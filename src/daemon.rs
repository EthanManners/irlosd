//! `irlosd serve`: supervise IrlosStudio, hold its one ICP connection, follow the
//! feed, and serve ICP to clients (the console today, the web panel later).

use crate::config::Config;
use crate::feed::{self, Debounce, Health};
use crate::{icp, studio};
use serde_json::{Value, json};
use signal_hook::consts::{SIGINT, SIGTERM};
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::process;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const STUDIO_TIMEOUT: Duration = Duration::from_secs(6);

macro_rules! log {
    ($($arg:tt)*) => { println!("irlosd: {}", format!($($arg)*)) };
}

struct Daemon {
    cfg: Config,
    studio: Mutex<Option<icp::Client>>,
    status: Mutex<Status>,
    want_streaming: AtomicBool,
    retry: Mutex<Retry>,
    feed_health: Mutex<Option<Health>>,
    shutting_down: AtomicBool,
    pid: AtomicI32,
}

#[derive(Default)]
struct Status {
    ready: bool,
    hello: Value,
    scene: Value,
    stream: Value,
    feed: Value,
    output_kbps: Option<i64>,
    last_stats: Option<(i64, i64)>,
    restarts: u32,
}

/// Backoff for restarting a stream that should be live but isn't.
struct Retry {
    next: Instant,
    delay: Duration,
}

pub fn serve() -> Result<(), String> {
    let mut cfg = Config::from_env()?;
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&cfg.run_dir)
        .map_err(|e| e.to_string())?;
    let socket = cfg.socket();
    if UnixStream::connect(&socket).is_ok() {
        return Err(format!(
            "another irlosd is already serving {}",
            socket.display()
        ));
    }
    let _ = fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket).map_err(|e| format!("{}: {e}", socket.display()))?;
    if cfg.stream_key.is_empty() {
        log!("STREAM_KEY is not set in .env; streaming is disabled until it is");
    }

    let sls = cfg.sls.take().map(feed::Poller::new);
    let d = Arc::new(Daemon {
        cfg,
        studio: Mutex::new(None),
        status: Mutex::new(Status::default()),
        want_streaming: AtomicBool::new(false),
        retry: Mutex::new(Retry {
            next: Instant::now(),
            delay: Duration::from_secs(5),
        }),
        feed_health: Mutex::new(None),
        shutting_down: AtomicBool::new(false),
        pid: AtomicI32::new(0),
    });
    log!(
        "irlosd {} listening on {}",
        env!("CARGO_PKG_VERSION"),
        socket.display()
    );

    let me = d.clone();
    let mut signals =
        signal_hook::iterator::Signals::new([SIGTERM, SIGINT]).map_err(|e| e.to_string())?;
    thread::spawn(move || {
        for _ in signals.forever() {
            me.shutdown();
        }
    });
    let me = d.clone();
    thread::spawn(move || me.supervise());
    let me = d.clone();
    thread::spawn(move || me.observe());
    if let Some(poller) = sls {
        let me = d.clone();
        thread::spawn(move || me.follow_feed(poller));
    } else {
        log!("SLS_STATS_URL is not set; automatic scene switching is off");
    }
    for conn in listener.incoming().flatten() {
        let me = d.clone();
        thread::spawn(move || me.serve_client(conn));
    }
    Ok(())
}

impl Daemon {
    /// One request to IrlosStudio over the single, serialized connection.
    fn studio(&self, req: &Value) -> Value {
        let mut conn = self.studio.lock().unwrap();
        if conn.is_none() {
            match icp::Client::connect(&self.cfg.studio_socket(), STUDIO_TIMEOUT) {
                Ok(c) => *conn = Some(c),
                Err(_) => return icp::error("obs_unavailable", "IrlosStudio is not running yet."),
            }
        }
        match conn.as_mut().unwrap().request(req) {
            Ok(resp) => resp,
            Err(e) => {
                *conn = None;
                icp::error(
                    "obs_unavailable",
                    &format!("Lost the connection to IrlosStudio: {e}"),
                )
            }
        }
    }

    fn supervise(&self) {
        let mut delay = Duration::from_secs(1);
        while !self.shutting_down.load(SeqCst) {
            if let Err(e) = studio::write_config(&self.cfg) {
                log!("cannot write the IrlosStudio config: {e}");
                process::exit(1);
            }
            let started = Instant::now();
            match studio::spawn(&self.cfg) {
                Ok(mut child) => {
                    self.pid.store(child.id() as i32, SeqCst);
                    log!("IrlosStudio started (pid {})", child.id());
                    let status = child.wait();
                    self.pid.store(0, SeqCst);
                    if self.shutting_down.load(SeqCst) {
                        log!("IrlosStudio stopped ({})", fmt_exit(status));
                        break;
                    }
                    log!(
                        "IrlosStudio exited ({}); restarting in {}s",
                        fmt_exit(status),
                        delay.as_secs()
                    );
                }
                Err(e) => log!(
                    "cannot launch {}: {e}; retrying in {}s",
                    self.cfg.obs.display(),
                    delay.as_secs()
                ),
            }
            self.status.lock().unwrap().restarts += 1;
            if started.elapsed() > Duration::from_secs(60) {
                delay = Duration::from_secs(1);
            }
            self.sleep(delay);
            delay = (delay * 2).min(Duration::from_secs(30));
        }
        let _ = fs::remove_file(self.cfg.socket());
        log!("bye");
        process::exit(0);
    }

    /// Polls IrlosStudio once a second and keeps the stream where it was asked to be.
    fn observe(&self) {
        loop {
            let scene = self.studio(&json!({ "op": "get_current_scene" }));
            let ready = scene["ok"] == true;
            let (stream, stats) = if ready {
                (
                    self.studio(&json!({ "op": "get_streaming_state" })),
                    self.studio(&json!({ "op": "get_stats" })),
                )
            } else {
                (Value::Null, Value::Null)
            };
            let became_ready = {
                let mut st = self.status.lock().unwrap();
                let became_ready = ready && !st.ready;
                if became_ready {
                    st.hello = self
                        .studio
                        .lock()
                        .unwrap()
                        .as_ref()
                        .map(|c| c.hello.clone())
                        .unwrap_or_default();
                }
                st.ready = ready;
                st.scene = scene["result"]["scene"].clone();
                st.stream = stream["result"].clone();
                st.output_kbps = None;
                let sample = stats["result"]["timestamp_ms"]
                    .as_i64()
                    .zip(stats["result"]["stream"]["total_bytes"].as_i64());
                if let (Some((t0, b0)), Some((t1, b1))) = (st.last_stats, sample)
                    && t1 > t0
                    && b1 >= b0
                {
                    st.output_kbps = Some((b1 - b0) * 8 / (t1 - t0));
                }
                st.last_stats = sample;
                became_ready
            };
            if became_ready {
                log!("IrlosStudio is ready (scene {})", scene["result"]["scene"]);
                if let Some(h) = *self.feed_health.lock().unwrap() {
                    self.switch(h.scene());
                }
            }
            match stream["result"]["state"].as_str() {
                Some("active") => self.retry.lock().unwrap().delay = Duration::from_secs(5),
                Some("inactive") if self.want_streaming.load(SeqCst) => {
                    let mut retry = self.retry.lock().unwrap();
                    if Instant::now() >= retry.next {
                        match stream["result"]["last_error"]
                            .as_str()
                            .filter(|e| !e.is_empty())
                        {
                            Some(why) => log!("stream is not live ({why}); starting it"),
                            None => log!("starting the stream"),
                        }
                        retry.next = Instant::now() + retry.delay;
                        retry.delay = (retry.delay * 2).min(Duration::from_secs(60));
                        drop(retry);
                        self.studio(&json!({ "op": "start_streaming" }));
                    }
                }
                _ => {}
            }
            thread::sleep(Duration::from_secs(1));
        }
    }

    fn follow_feed(&self, poller: feed::Poller) {
        let mut debounce = Debounce::default();
        loop {
            let sample = poller.poll();
            let health = poller.classify(&sample);
            self.status.lock().unwrap().feed = feed::describe(&sample, health.or(debounce.settled));
            if let Some(h) = debounce.push(health) {
                *self.feed_health.lock().unwrap() = Some(h);
                // While IrlosStudio is down, the switch happens when it becomes ready.
                if self.status.lock().unwrap().ready {
                    log!("feed is {h:?}; switching to {}", h.scene());
                    self.switch(h.scene());
                }
            }
            thread::sleep(Duration::from_secs(1));
        }
    }

    fn switch(&self, scene: &str) {
        let resp = self.studio(&json!({ "op": "switch_scene", "scene": scene }));
        if resp["ok"] != true {
            log!("cannot switch to {scene}: {}", icp::describe_error(&resp));
        }
    }

    fn serve_client(&self, mut conn: UnixStream) {
        let _ = conn.set_write_timeout(Some(Duration::from_secs(5)));
        let hello = json!({ "protocol_version": icp::PROTOCOL_VERSION, "irlosd_version": env!("CARGO_PKG_VERSION") });
        if !matches!(icp::accept_hello(&mut conn, hello), Ok(true)) {
            return;
        }
        while let Ok(req) = icp::read_frame(&mut conn) {
            if icp::write_frame(&mut conn, &self.handle(&req)).is_err() {
                return;
            }
        }
    }

    fn handle(&self, req: &Value) -> Value {
        match req["op"].as_str() {
            Some("get_status") => icp::ok(self.status_json()),
            Some("start_streaming") if self.cfg.stream_key.is_empty() => icp::error(
                "no_stream_key",
                "Put STREAM_KEY in .env, then run ./irlosd again.",
            ),
            Some("start_streaming") => {
                self.want_streaming.store(true, SeqCst);
                log!("start requested");
                let ready = self.status.lock().unwrap().ready;
                let wait = if ready {
                    Duration::from_secs(5)
                } else {
                    Duration::ZERO
                };
                *self.retry.lock().unwrap() = Retry {
                    next: Instant::now() + wait,
                    delay: Duration::from_secs(5),
                };
                if !ready {
                    return icp::error(
                        "obs_unavailable",
                        "IrlosStudio is still starting; the stream starts as soon as it is ready.",
                    );
                }
                self.studio(req)
            }
            Some("stop_streaming") => {
                self.want_streaming.store(false, SeqCst);
                log!("stop requested");
                self.studio(req)
            }
            Some("hello") => icp::error("malformed_request", "Already handshaken."),
            Some(_) => self.studio(req),
            None => icp::error("malformed_request", "Missing op."),
        }
    }

    fn status_json(&self) -> Value {
        let st = self.status.lock().unwrap();
        json!({
            "irlosd_version": env!("CARGO_PKG_VERSION"),
            "studio": if st.ready { "ready" } else if self.pid.load(SeqCst) > 0 { "starting" } else { "down" },
            "studio_versions": st.hello,
            "studio_restarts": st.restarts,
            "scene": st.scene,
            "stream": st.stream,
            "want_streaming": self.want_streaming.load(SeqCst),
            "output_kbps": st.output_kbps,
            "feed": st.feed,
        })
    }

    fn shutdown(&self) {
        if self.shutting_down.swap(true, SeqCst) {
            process::exit(130);
        }
        log!("shutting down");
        let pid = self.pid.load(SeqCst);
        if pid <= 0 {
            let _ = fs::remove_file(self.cfg.socket());
            process::exit(0);
        }
        // IrlosStudio drains its outputs on SIGTERM and exits within a few seconds.
        unsafe { libc::kill(pid, libc::SIGTERM) };
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(10));
            log!("IrlosStudio did not stop in 10s; killing it");
            unsafe { libc::kill(pid, libc::SIGKILL) };
            thread::sleep(Duration::from_secs(2));
            process::exit(1);
        });
    }

    fn sleep(&self, total: Duration) {
        let until = Instant::now() + total;
        while Instant::now() < until && !self.shutting_down.load(SeqCst) {
            thread::sleep(Duration::from_millis(100));
        }
    }
}

fn fmt_exit(status: std::io::Result<process::ExitStatus>) -> String {
    match status {
        Ok(s) => s.to_string(),
        Err(e) => e.to_string(),
    }
}
