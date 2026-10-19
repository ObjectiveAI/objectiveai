//! The daemon's own handshake toward the database, as the
//! container's role.

use bytes::{Bytes, BytesMut};
use fallible_iterator::FallibleIterator as _;
use postgres_protocol::authentication::md5_hash;
use postgres_protocol::authentication::sasl::{ChannelBinding, ScramSha256};
use postgres_protocol::message::backend::{Header, Message};
use postgres_protocol::message::frontend;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use super::Server;
use crate::database::{Scope, Target};

/// The mechanism the daemon speaks.
const SCRAM_SHA_256: &str = "SCRAM-SHA-256";

/// The handshake done: what the database said after authenticating,
/// to be forwarded to the container as it was — every
/// `ParameterStatus` and notice, the `BackendKeyData`, the
/// `ReadyForQuery` — and the backend key, for a cancel.
pub struct Established {
    /// The messages, each whole, in order.
    pub replay: Vec<Bytes>,
    /// The session's process id and secret key.
    pub backend: Option<(i32, i32)>,
}

/// Why the handshake did not complete: the server's own
/// `ErrorResponse`, whole, when it sent one, and why in a sentence.
pub struct Failure {
    /// The server's error, to forward.
    pub response: Option<Bytes>,
    /// Why.
    pub why: String,
}

impl Failure {
    fn new(why: impl Into<String>) -> Failure {
        Failure {
            response: None,
            why: why.into(),
        }
    }
}

/// Send the daemon's startup message — the scope's role, the target's
/// database, and the container's parameters passed through — and
/// answer what the server asks: SCRAM-SHA-256, md5, or cleartext over
/// TLS alone; read on to `ReadyForQuery`, keeping what the server said
/// for the container.
pub async fn handshake(
    server: &mut Server,
    encrypted: bool,
    target: &Target,
    scope: &Scope,
    parameters: Vec<(String, String)>,
) -> Result<Established, Failure> {
    let mut startup = BytesMut::new();
    let named = [("user".to_string(), scope.role.clone()), ("database".to_string(), target.dbname.clone())];
    frontend::startup_message(
        named.iter().chain(parameters.iter()).map(|(name, value)| (name.as_str(), value.as_str())),
        &mut startup,
    )
    .map_err(|error| Failure::new(format!("the startup message could not be written: {error}")))?;
    write(server, &startup).await?;
    let mut buffer = BytesMut::with_capacity(8 * 1024);
    let mut scram: Option<ScramSha256> = None;
    let mut established = Established {
        replay: Vec::new(),
        backend: None,
    };
    loop {
        let (raw, message) = next(server, &mut buffer).await?;
        match message {
            Message::AuthenticationOk => {}
            Message::AuthenticationSasl(body) => {
                let mut mechanisms = body.mechanisms();
                let mut offered: Vec<String> = Vec::new();
                while let Some(mechanism) = mechanisms.next().map_err(|error| Failure::new(error.to_string()))? {
                    offered.push(mechanism.to_string());
                }
                if !offered.iter().any(|mechanism| mechanism == SCRAM_SHA_256) {
                    return Err(Failure::new(format!("the database offers none of the daemon's SASL mechanisms: {offered:?}")));
                }
                let exchange = ScramSha256::new(scope.password.as_bytes(), ChannelBinding::unsupported());
                let mut response = BytesMut::new();
                frontend::sasl_initial_response(SCRAM_SHA_256, exchange.message(), &mut response)
                    .map_err(|error| Failure::new(error.to_string()))?;
                write(server, &response).await?;
                scram = Some(exchange);
            }
            Message::AuthenticationSaslContinue(body) => {
                let exchange = scram.as_mut().ok_or_else(|| Failure::new("SASL continued before it began"))?;
                exchange
                    .update(body.data())
                    .map_err(|error| Failure::new(format!("SCRAM failed: {error}")))?;
                let mut response = BytesMut::new();
                frontend::sasl_response(exchange.message(), &mut response).map_err(|error| Failure::new(error.to_string()))?;
                write(server, &response).await?;
            }
            Message::AuthenticationSaslFinal(body) => {
                let exchange = scram.as_mut().ok_or_else(|| Failure::new("SASL finished before it began"))?;
                exchange
                    .finish(body.data())
                    .map_err(|error| Failure::new(format!("the database's SCRAM signature did not verify: {error}")))?;
            }
            Message::AuthenticationMd5Password(body) => {
                let hashed = md5_hash(scope.role.as_bytes(), scope.password.as_bytes(), body.salt());
                let mut response = BytesMut::new();
                frontend::password_message(hashed.as_bytes(), &mut response).map_err(|error| Failure::new(error.to_string()))?;
                write(server, &response).await?;
            }
            Message::AuthenticationCleartextPassword => {
                if !encrypted {
                    return Err(Failure::new("the database asks for a cleartext password over a connection that is not encrypted"));
                }
                let mut response = BytesMut::new();
                frontend::password_message(scope.password.as_bytes(), &mut response).map_err(|error| Failure::new(error.to_string()))?;
                write(server, &response).await?;
            }
            Message::ParameterStatus(_) | Message::BackendKeyData(_) | Message::NoticeResponse(_) => {
                if let Message::BackendKeyData(body) = &message {
                    established.backend = Some((body.process_id(), body.secret_key()));
                }
                established.replay.push(raw);
            }
            Message::ReadyForQuery(_) => {
                established.replay.push(raw);
                return Ok(established);
            }
            Message::ErrorResponse(body) => {
                let mut fields = body.fields();
                let mut why = None;
                while let Ok(Some(field)) = fields.next() {
                    if field.type_() == b'M' {
                        why = Some(String::from_utf8_lossy(field.value_bytes()).into_owned());
                        break;
                    }
                }
                let why = why.unwrap_or_else(|| "the database refused the connection".to_string());
                return Err(Failure {
                    response: Some(raw),
                    why,
                });
            }
            _ => return Err(Failure::new("the database asks for an authentication the daemon does not speak")),
        }
    }
}

/// Write one message, whole.
async fn write(server: &mut Server, bytes: &[u8]) -> Result<(), Failure> {
    server
        .write_all(bytes)
        .await
        .map_err(|error| Failure::new(format!("the database connection failed: {error}")))
}

/// The next message the server sends, whole: its bytes as they came,
/// and parsed.
async fn next(server: &mut Server, buffer: &mut BytesMut) -> Result<(Bytes, Message), Failure> {
    loop {
        if let Some(header) = Header::parse(buffer).map_err(|error| Failure::new(error.to_string()))? {
            let whole = usize::try_from(header.len()).unwrap_or(0) + 1;
            if buffer.len() >= whole {
                let raw = Bytes::copy_from_slice(&buffer[..whole]);
                let message = Message::parse(buffer)
                    .map_err(|error| Failure::new(error.to_string()))?
                    .ok_or_else(|| Failure::new("a message the parser would not take whole"))?;
                return Ok((raw, message));
            }
        }
        let read = server
            .read_buf(buffer)
            .await
            .map_err(|error| Failure::new(format!("the database connection failed: {error}")))?;
        if read == 0 {
            return Err(Failure::new("the database closed the connection during the handshake"));
        }
    }
}
