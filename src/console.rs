//! `irlosd` / `irlosd console`: a one-screen control panel over ICP.
//! Quitting the console leaves the daemon and the stream running.

use crate::icp;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::{cursor, execute, queue, style::Print, terminal};
use serde_json::{Value, json};
use std::io::{self, Write};
use std::time::Duration;

pub fn run(mut client: icp::Client) -> Result<(), String> {
    let mut out = io::stdout();
    terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    let _ = execute!(out, terminal::EnterAlternateScreen, cursor::Hide);
    let result = control(&mut client, &mut out);
    let _ = execute!(out, cursor::Show, terminal::LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
    result.map_err(|e| e.to_string())?;
    println!("irlosd keeps running. Open this console again with ./irlosd");
    Ok(())
}

fn control(client: &mut icp::Client, out: &mut impl Write) -> io::Result<()> {
    let mut message = String::from("Press s to go live.");
    loop {
        let status = client.request(&json!({ "op": "get_status" }))?;
        draw(out, &status["result"], &message)?;
        if !event::poll(Duration::from_secs(1))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let (req, verb) = match key.code {
            KeyCode::Char('s') => (json!({ "op": "start_streaming" }), "Starting the stream"),
            KeyCode::Char('x') => (json!({ "op": "stop_streaming" }), "Stopping the stream"),
            KeyCode::Char('l') => (
                json!({ "op": "switch_scene", "scene": "Live" }),
                "Switched to Live",
            ),
            KeyCode::Char('w') => (
                json!({ "op": "switch_scene", "scene": "Low" }),
                "Switched to Low",
            ),
            KeyCode::Char('b') => (
                json!({ "op": "switch_scene", "scene": "BRB" }),
                "Switched to BRB",
            ),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            _ => continue,
        };
        let resp = client.request(&req)?;
        message = match resp["error"]["message"].as_str() {
            None => format!("{verb}."),
            Some(why) => why.to_string(),
        };
    }
}

fn draw(out: &mut impl Write, st: &Value, message: &str) -> io::Result<()> {
    let text = |v: &Value| v.as_str().unwrap_or("-").to_string();
    let studio = match (
        st["studio"].as_str(),
        st["studio_versions"]["irlosstudio_version"].as_str(),
    ) {
        (Some("ready"), Some(v)) => format!(
            "ready  (IrlosStudio {v}, OBS {})",
            text(&st["studio_versions"]["obs_version"])
        ),
        (s, _) => s.unwrap_or("-").to_string(),
    };
    let stream = &st["stream"];
    let mut live = match stream["state"].as_str() {
        Some("active") => format!(
            "LIVE  {}",
            clock(stream["duration_ms"].as_i64().unwrap_or(0))
        ),
        Some(s) => s.to_string(),
        None => "-".into(),
    };
    if let Some(kbps) = st["output_kbps"].as_i64() {
        live += &format!("  {kbps} kbps out");
    }
    if let Some(e) = stream["last_error"].as_str().filter(|e| !e.is_empty()) {
        live += &format!("  ({e})");
    }
    let feed = &st["feed"];
    let feed = match (feed["health"].as_str(), feed["bitrate_kbps"].as_i64()) {
        (Some(h), Some(b)) => format!("{h}  {b} kbps  {} ms rtt", feed["rtt_ms"]),
        (Some(h), None) => format!("{h}  ({})", text(&feed["detail"])),
        _ => "no SLS stats configured".into(),
    };
    let lines = [
        format!("IRLOS  irlosd {}", text(&st["irlosd_version"])),
        String::new(),
        format!("  Studio   {studio}"),
        format!("  Stream   {live}"),
        format!("  Scene    {}", text(&st["scene"])),
        format!("  Feed     {feed}"),
        String::new(),
        "  [s] start   [x] stop   [l] Live   [w] Low   [b] BRB   [q] quit".into(),
        String::new(),
        format!("  {message}"),
    ];
    queue!(
        out,
        cursor::MoveTo(0, 0),
        terminal::Clear(terminal::ClearType::All)
    )?;
    for line in lines {
        queue!(out, Print(line), Print("\r\n"))?;
    }
    out.flush()
}

fn clock(ms: i64) -> String {
    let s = ms / 1000;
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}
