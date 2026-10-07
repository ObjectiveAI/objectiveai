//! The container's first packet, and the daemon's first answers.
//!
//! pgwire's first message in each direction carries no type byte: a
//! length, a code, and for a startup message the parameters. The
//! `postgres-protocol` crate builds these and does not read them, so
//! the one parse the daemon does itself is here, and it is the whole
//! of what the daemon ever reads of a container's traffic.

use bytes::{Bytes, BytesMut};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// The code of an `SSLRequest`.
const SSL_REQUEST: i32 = 80877103;

/// The code of a `GSSENCRequest`.
const GSSENC_REQUEST: i32 = 80877104;

/// The code of a `CancelRequest`.
const CANCEL_REQUEST: i32 = 80877102;

/// Protocol 3.0, the code of a `StartupMessage`.
const PROTOCOL_3_0: i32 = 196608;

/// The one message the daemon writes toward a container of its own
/// accord: `AuthenticationOk`, the type `R`, length eight, and zero.
pub const AUTHENTICATION_OK: [u8; 9] = [b'R', 0, 0, 0, 8, 0, 0, 0, 0];

/// What the container's first packet was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    /// A startup message: its parameters, in order, with `user`,
    /// `database`, `replication` and `options` taken out — those are
    /// the daemon's or nobody's.
    Startup(Vec<(String, String)>),
    /// A cancel request for a backend the container's session was
    /// given.
    Cancel {
        /// The backend's process id.
        process_id: i32,
        /// Its secret key.
        secret_key: i32,
    },
}

/// Read the container's first packet: an `SSLRequest` or a
/// `GSSENCRequest` answered `N` and read past, a `CancelRequest` or a
/// `StartupMessage` the answer. None for the container gone first, or
/// a packet that is none of these.
pub async fn read_startup(from: &mut UnboundedReceiver<Bytes>, to: &UnboundedSender<Bytes>) -> Option<Packet> {
    let mut buffer = BytesMut::new();
    loop {
        while buffer.len() >= 4 {
            let length = i32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
            let length = usize::try_from(length).ok()?;
            if length < 8 {
                return None;
            }
            if buffer.len() < length {
                break;
            }
            let packet = buffer.split_to(length);
            let code = i32::from_be_bytes([packet[4], packet[5], packet[6], packet[7]]);
            match code {
                SSL_REQUEST | GSSENC_REQUEST => {
                    to.send(Bytes::from_static(b"N")).ok()?;
                }
                CANCEL_REQUEST => {
                    if packet.len() < 16 {
                        return None;
                    }
                    return Some(Packet::Cancel {
                        process_id: i32::from_be_bytes([packet[8], packet[9], packet[10], packet[11]]),
                        secret_key: i32::from_be_bytes([packet[12], packet[13], packet[14], packet[15]]),
                    });
                }
                PROTOCOL_3_0 => return Some(Packet::Startup(parameters(&packet[8..]))),
                _ => return None,
            }
        }
        let bytes = from.recv().await?;
        buffer.extend_from_slice(&bytes);
    }
}

/// The parameters of a startup message: NUL-terminated names and
/// values, alternating, ended by an empty name; the four that are not
/// the container's to set left out.
fn parameters(body: &[u8]) -> Vec<(String, String)> {
    let mut parameters = Vec::new();
    let mut fields = body.split(|byte| *byte == 0);
    while let Some(name) = fields.next() {
        if name.is_empty() {
            break;
        }
        let Some(value) = fields.next() else {
            break;
        };
        let name = String::from_utf8_lossy(name).into_owned();
        if matches!(name.as_str(), "user" | "database" | "replication" | "options") {
            continue;
        }
        parameters.push((name, String::from_utf8_lossy(value).into_owned()));
    }
    parameters
}

/// An `ErrorResponse` of the daemon's own: `FATAL`, the SQLSTATE
/// `code`, and `message` — what a container's driver is told when the
/// connection could not be made, in the form it reads.
pub fn error_response(code: &str, message: &str) -> Bytes {
    let mut body = Vec::new();
    for (field, value) in [(b'S', "FATAL"), (b'V', "FATAL"), (b'C', code), (b'M', message)] {
        body.push(field);
        body.extend_from_slice(value.as_bytes());
        body.push(0);
    }
    body.push(0);
    let mut message = Vec::with_capacity(5 + body.len());
    message.push(b'E');
    message.extend_from_slice(&(u32::try_from(body.len() + 4).unwrap_or(u32::MAX)).to_be_bytes());
    message.extend_from_slice(&body);
    Bytes::from(message)
}
