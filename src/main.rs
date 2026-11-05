mod config;
mod console;
mod daemon;
mod feed;
mod icp;
mod studio;

use serde_json::{Value, json};
use std::process::exit;
use std::time::Duration;

const USAGE: &str = "usage: irlosd [console | serve | status | start | stop | scene <name>]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        [] | ["console"] => connect().and_then(console::run),
        ["serve"] => daemon::serve(),
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
