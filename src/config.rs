//! Settings come from the environment (`.env`, passed in by docker compose).

use std::env;
use std::path::PathBuf;

/// Kick's ingest server. Any RTMP(S) server works through `STREAM_SERVER`.
const KICK: &str = "rtmps://fa723fc1b171.global-contribute.live-video.net:443/app/";

pub struct Config {
    pub run_dir: PathBuf,
    pub state_dir: PathBuf,
    pub template_dir: PathBuf,
    pub obs: PathBuf,
    pub stream_key: String,
    /// Values substituted for `{{NAME}}` in the IrlosStudio templates.
    pub vars: Vec<(&'static str, String)>,
    pub sls: Option<Sls>,
}

pub struct Sls {
    pub stats_url: String,
    pub publisher: String,
    pub low_kbps: i64,
    pub low_rtt_ms: f64,
    pub offline_kbps: i64,
}

impl Config {
    pub fn from_env() -> Result<Config, String> {
        let (width, height) = var("RESOLUTION", "1920x1080")
            .split_once('x')
            .and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)))
            .filter(|&(w, h)| {
                (2..=4096).contains(&w) && (2..=4096).contains(&h) && w % 2 == 0 && h % 2 == 0
            })
            .ok_or("RESOLUTION must look like 1920x1080")?;
        let fps: u32 = number("FPS", "30", 1..=120)?;
        let bitrate: u32 = number("BITRATE_KBPS", "6000", 500..=50000)?;
        let stream_key = var("STREAM_KEY", "");
        let sls = match var("SLS_STATS_URL", "") {
            url if url.is_empty() => None,
            stats_url => Some(Sls {
                stats_url,
                publisher: var("SLS_PUBLISHER", "publish/live/feed1"),
                low_kbps: number("LOW_KBPS", "800", 0..=100_000)?,
                low_rtt_ms: number::<u32>("LOW_RTT_MS", "2500", 0..=60_000)?.into(),
                offline_kbps: number("OFFLINE_KBPS", "0", 0..=100_000)?,
            }),
        };
        Ok(Config {
            run_dir: var("IRLOS_RUN_DIR", "/run/irlos").into(),
            state_dir: var("IRLOS_STATE_DIR", "/var/lib/irlos").into(),
            template_dir: var("IRLOS_TEMPLATE_DIR", "/usr/share/irlosd/studio").into(),
            obs: var("IRLOS_OBS", "/usr/bin/obs").into(),
            vars: vec![
                ("STREAM_SERVER", var("STREAM_SERVER", KICK)),
                ("STREAM_KEY", stream_key.clone()),
                ("SRT_URL", var("SRT_URL", "")),
                ("WIDTH", width.to_string()),
                ("HEIGHT", height.to_string()),
                ("FPS", fps.to_string()),
                ("BITRATE_KBPS", bitrate.to_string()),
                ("BRB_MEDIA", var("BRB_MEDIA", "/usr/share/irlosd/brb.mp4")),
                ("LOW_MEDIA", var("LOW_MEDIA", "/usr/share/irlosd/low.png")),
            ],
            stream_key,
            sls,
        })
    }

    pub fn socket(&self) -> PathBuf {
        self.run_dir.join("irlosd.sock")
    }

    pub fn studio_socket(&self) -> PathBuf {
        self.run_dir.join("obs-control.sock")
    }
}

/// Where clients find the daemon, without needing a full config.
pub fn client_socket() -> PathBuf {
    PathBuf::from(var("IRLOS_RUN_DIR", "/run/irlos")).join("irlosd.sock")
}

fn var(name: &str, default: &str) -> String {
    env::var(name)
        .map(|v| v.trim().to_string())
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or(default.into())
}

fn number<T>(name: &str, default: &str, range: std::ops::RangeInclusive<T>) -> Result<T, String>
where
    T: std::str::FromStr + PartialOrd + std::fmt::Debug,
{
    var(name, default)
        .parse::<T>()
        .ok()
        .filter(|v| range.contains(v))
        .ok_or_else(|| format!("{name} must be a whole number in {range:?}"))
}
