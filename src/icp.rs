//! ICP (IRLOS Control Protocol) v1: length-prefixed JSON over a Unix socket.
//! See IrlosStudio/plugins/irlos-control/protocol.md. irlosd speaks it twice:
//! as a client of IrlosStudio, and as a server for its own clients.

use serde_json::{Value, json};
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

pub const PROTOCOL_VERSION: i64 = 1;
const MAX_PAYLOAD: usize = 16384;

pub fn read_frame(r: &mut impl Read) -> io::Result<Value> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len)?;
    let n = u32::from_be_bytes(len) as usize;
    if !(2..=MAX_PAYLOAD).contains(&n) {
        return Err(invalid("frame length out of range"));
    }
    let mut buf = vec![0; n];
    r.read_exact(&mut buf)?;
    match serde_json::from_slice::<Value>(&buf) {
        Ok(v) if v.is_object() => Ok(v),
        _ => Err(invalid("payload is not a JSON object")),
    }
}

pub fn write_frame(w: &mut impl Write, v: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(v)?;
    if body.len() > MAX_PAYLOAD {
        return Err(invalid("message too large"));
    }
    let mut frame = (body.len() as u32).to_be_bytes().to_vec();
    frame.extend_from_slice(&body);
    w.write_all(&frame)
}

pub fn ok(result: Value) -> Value {
    json!({ "ok": true, "result": result })
}

pub fn error(code: &str, message: &str) -> Value {
    json!({ "ok": false, "error": { "code": code, "message": message } })
}

/// `"code: message"` for an error response, `""` for a success.
pub fn describe_error(resp: &Value) -> String {
    match resp["ok"].as_bool() {
        Some(true) => String::new(),
        _ => format!(
            "{}: {}",
            resp["error"]["code"].as_str().unwrap_or("bad_response"),
            resp["error"]["message"].as_str().unwrap_or("")
        ),
    }
}

/// One handshaken ICP connection. Requests are strictly one at a time.
pub struct Client {
    stream: UnixStream,
    pub hello: Value,
}

impl Client {
    pub fn connect(path: &Path, timeout: Duration) -> io::Result<Client> {
        let stream = UnixStream::connect(path)?;
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;
        let mut client = Client {
            stream,
            hello: Value::Null,
        };
        let resp =
            client.request(&json!({ "op": "hello", "protocol_version": PROTOCOL_VERSION }))?;
        if resp["ok"] != true {
            return Err(io::Error::other(describe_error(&resp)));
        }
        client.hello = resp["result"].clone();
        Ok(client)
    }

    pub fn request(&mut self, req: &Value) -> io::Result<Value> {
        write_frame(&mut self.stream, req)?;
        read_frame(&mut self.stream)
    }
}

/// Server side of the handshake. Answers the client's `hello` and reports whether it succeeded.
pub fn accept_hello(stream: &mut UnixStream, result: Value) -> io::Result<bool> {
    let hello = read_frame(stream)?;
    let reply = if hello["op"] != "hello" {
        error("not_handshaked", "The first request must be hello.")
    } else if !hello["protocol_version"].is_i64() {
        error("malformed_hello", "protocol_version must be an integer.")
    } else if hello["protocol_version"] != PROTOCOL_VERSION {
        error(
            "version_mismatch",
            "This server implements protocol version 1.",
        )
    } else {
        ok(result)
    };
    write_frame(stream, &reply)?;
    Ok(reply["ok"] == true)
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_round_trip() {
        let mut buf = Vec::new();
        write_frame(&mut buf, &json!({ "op": "get_current_scene" })).unwrap();
        assert_eq!(&buf[..4], &(buf.len() as u32 - 4).to_be_bytes());
        assert_eq!(
            read_frame(&mut buf.as_slice()).unwrap()["op"],
            "get_current_scene"
        );
    }

    #[test]
    fn rejects_bad_frames() {
        for bad in [
            &[0, 0, 0, 1, b'{'][..],
            &[0, 0, 0x40, 1][..],
            &[0, 0, 0, 2, b'[', b']'][..],
        ] {
            assert!(read_frame(&mut &bad[..]).is_err());
        }
    }

    #[test]
    fn handshake() {
        for (hello, ok) in [
            (json!({ "op": "hello", "protocol_version": 1 }), true),
            (json!({ "op": "hello", "protocol_version": 1.0 }), false),
            (json!({ "op": "hello", "protocol_version": 2 }), false),
            (json!({ "op": "get_status" }), false),
        ] {
            let (mut a, mut b) = UnixStream::pair().unwrap();
            write_frame(&mut a, &hello).unwrap();
            assert_eq!(accept_hello(&mut b, json!({})).unwrap(), ok);
            assert_eq!(read_frame(&mut a).unwrap()["ok"], ok);
        }
    }
}
