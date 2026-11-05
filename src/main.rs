mod config;
mod console;
mod daemon;
mod feed;
mod icp;
mod studio;

use serde_json::{Value, json};
use std::process::exit;
use std::time::Duration;

const USAGE: &str =
    "usage: irlosd [console | serve | status | start | stop | scene <name> | bridge]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        [] | ["console"] => connect().and_then(console::run),
        ["serve"] => daemon::serve(),
        ["bridge"] => bridge(),
        ["status"] => once(json!({ "op": "get_status" })),
        ["start"] => once(json!({ "op": "start_streaming" })),
        ["stop"] => once(json!({ "op": "stop_streaming" })),
        ["scene", name] => once(json!({ "op": "switch_scene", "scene": name })),
        ["-V" | "--version"] => {
            println!("irlosd {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => {
            eprintln!("{USAGE}");
            exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("irlosd: {e}");
        exit(1);
    }
}

/// Connect to the daemon, giving a just-launched one a few seconds to come up.
fn connect() -> Result<icp::Client, String> {
    let socket = config::client_socket();
    let mut tries = 0;
    loop {
        match icp::Client::connect(&socket, Duration::from_secs(10)) {
            Ok(client) => return Ok(client),
            Err(_) if tries < 20 => tries += 1,
            Err(e) => {
                return Err(format!(
                    "cannot reach irlosd at {} ({e}). Is it running?",
                    socket.display()
                ));
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Send one ICP request to the daemon and print the response.
fn once(req: Value) -> Result<(), String> {
    let mut client = connect()?;
    let resp = client.request(&req).map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&resp).unwrap_or_default()
    );
    match icp::describe_error(&resp) {
        e if e.is_empty() => Ok(()),
        e => Err(e),
    }
}

/// Pipe stdin/stdout to the daemon socket, raw. This is how remote clients
/// (the irlos.live backend, over a forced-command SSH key) speak ICP to irlosd.
fn bridge() -> Result<(), String> {
    use std::io::{Read, Write};
    let socket = config::client_socket();
    let mut to_daemon = std::os::unix::net::UnixStream::connect(&socket)
        .map_err(|e| format!("cannot reach irlosd at {} ({e})", socket.display()))?;
    let mut from_daemon = to_daemon.try_clone().map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let mut buf = [0u8; 16384];
        let mut out = std::io::stdout().lock();
        while let Ok(n @ 1..) = from_daemon.read(&mut buf) {
            if out.write_all(&buf[..n]).and_then(|_| out.flush()).is_err() {
                break;
            }
        }
        exit(0);
    });
    let _ = std::io::copy(&mut std::io::stdin().lock(), &mut to_daemon);
    let _ = to_daemon.shutdown(std::net::Shutdown::Write);
    std::thread::sleep(Duration::from_secs(10));
    Ok(())
}
