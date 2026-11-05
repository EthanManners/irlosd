//! Field-feed health from SRT Live Server stats, and the debounce that turns
//! samples into scene changes. Thresholds follow NOALBS.

use crate::config::Sls;
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Health {
    Live,
    Low,
    Offline,
}

impl Health {
    pub fn scene(self) -> &'static str {
        match self {
            Health::Live => "Live",
            Health::Low => "Low",
            Health::Offline => "BRB",
        }
    }
}

pub struct Sample {
    pub bitrate_kbps: i64,
    pub rtt_ms: f64,
}

pub struct Poller {
    sls: Sls,
    agent: ureq::Agent,
}

impl Poller {
    pub fn new(sls: Sls) -> Poller {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_millis(750)))
            .build()
            .into();
        Poller { sls, agent }
    }

    /// `Ok(None)` when the stats page answers but our publisher isn't on it.
    pub fn poll(&self) -> Result<Option<Sample>, String> {
        let body = self
            .agent
            .get(&self.sls.stats_url)
            .call()
            .map_err(|e| e.to_string())?
            .body_mut()
            .with_config()
            .limit(1 << 20)
            .read_to_string()
            .map_err(|e| e.to_string())?;
        let stats: Value =
            serde_json::from_str(&body).map_err(|_| "stats are not JSON".to_string())?;
        let publisher = &stats["publishers"][&self.sls.publisher];
        Ok(publisher["bitrate"].as_i64().map(|bitrate_kbps| Sample {
            bitrate_kbps,
            rtt_ms: publisher["rtt"].as_f64().unwrap_or(0.0),
        }))
    }

    /// `None` means "no opinion" (publisher connected but not sending yet).
    pub fn classify(&self, sample: &Result<Option<Sample>, String>) -> Option<Health> {
        let Ok(Some(s)) = sample else {
            return Some(Health::Offline);
        };
        let t = &self.sls;
        if t.offline_kbps > 0 && s.bitrate_kbps > 0 && s.bitrate_kbps <= t.offline_kbps {
            Some(Health::Offline)
        } else if s.bitrate_kbps == 0 {
            None
        } else if s.bitrate_kbps <= t.low_kbps || (t.low_rtt_ms > 0.0 && s.rtt_ms >= t.low_rtt_ms) {
            Some(Health::Low)
        } else {
            Some(Health::Live)
        }
    }
}

pub fn describe(sample: &Result<Option<Sample>, String>, health: Option<Health>) -> Value {
    let health = health.map(|h| format!("{h:?}"));
    match sample {
        Ok(Some(s)) => {
            json!({ "health": health, "bitrate_kbps": s.bitrate_kbps, "rtt_ms": s.rtt_ms.round() as i64 })
        }
        Ok(None) => json!({ "health": "Offline", "detail": "no publisher" }),
        Err(e) => json!({ "health": "Offline", "detail": format!("stats unreachable: {e}") }),
    }
}

/// A new health has to hold for 3 samples to make things worse and 5 to make them better.
#[derive(Default)]
pub struct Debounce {
    pub settled: Option<Health>,
    candidate: Option<Health>,
    count: u32,
}

impl Debounce {
    /// Returns the new health when it settles.
    pub fn push(&mut self, health: Option<Health>) -> Option<Health> {
        let h = health?;
        if self.settled == Some(h) {
            self.candidate = None;
            return None;
        }
        if self.candidate == Some(h) {
            self.count += 1;
        } else {
            (self.candidate, self.count) = (Some(h), 1);
        }
        let needed = match self.settled {
            None => 1,
            Some(cur) if h > cur => 3,
            Some(_) => 5,
        };
        if self.count < needed {
            return None;
        }
        (self.settled, self.candidate) = (Some(h), None);
        Some(h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Health::*;

    #[test]
    fn debounce() {
        let mut d = Debounce::default();
        assert_eq!(d.push(Some(Live)), Some(Live));
        assert_eq!(d.push(Some(Offline)), None);
        assert_eq!(d.push(Some(Offline)), None);
        assert_eq!(d.push(None), None);
        assert_eq!(d.push(Some(Offline)), Some(Offline));
        for _ in 0..4 {
            assert_eq!(d.push(Some(Live)), None);
        }
        assert_eq!(d.push(Some(Live)), Some(Live));
        assert_eq!(d.push(Some(Low)), None);
        assert_eq!(d.push(Some(Live)), None);
        assert_eq!(d.push(Some(Low)), None);
    }

    #[test]
    fn classify() {
        let sls = Sls {
            stats_url: String::new(),
            publisher: String::new(),
            low_kbps: 800,
            low_rtt_ms: 2500.0,
            offline_kbps: 0,
        };
        let p = Poller::new(sls);
        let s = |bitrate_kbps, rtt_ms| {
            Ok(Some(Sample {
                bitrate_kbps,
                rtt_ms,
            }))
        };
        assert_eq!(p.classify(&s(5000, 40.0)), Some(Live));
        assert_eq!(p.classify(&s(700, 40.0)), Some(Low));
        assert_eq!(p.classify(&s(5000, 3000.0)), Some(Low));
        assert_eq!(p.classify(&s(0, 40.0)), None);
        assert_eq!(p.classify(&Ok(None)), Some(Offline));
        assert_eq!(p.classify(&Err("down".into())), Some(Offline));
    }
}
