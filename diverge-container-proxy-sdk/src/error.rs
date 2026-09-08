//! What can go wrong between a program and the proxy.

use std::error;
use std::fmt;

/// A failure of the client's own link to the proxy.
///
/// What a tool SAID is never here: the proxy turns a tool's refusal
/// into a result the program reads. Variants are named for the
/// feature they belong to, so a program handling one knows which
/// connection it was.
#[derive(Debug)]
pub enum Error {
    /// Dialing or initializing the MCP session failed.
    McpConnect(rmcp::service::ClientInitializeError),
    /// An MCP exchange could not be carried: the session is gone, or
    /// the proxy answered something rmcp could not read.
    McpService(rmcp::ServiceError),
    /// A vault key too long for its request's length prefix — only
    /// `set` has one.
    VaultKey(diverge_provider_sdk::container_proxy::vault::RequestEncodeError),
    /// The HTTP call to the proxy failed.
    VaultRequest(reqwest::Error),
    /// The proxy answered a status other than success: `400` for a
    /// request it could not read, `502` for an ask that died or that
    /// the caller refused unanswered. No retry is made — a vault
    /// operation is not safe to repeat blindly.
    VaultStatus(u16),
    /// The proxy's answer could not be read as the vault's.
    VaultAnswer(diverge_provider_sdk::container_proxy::vault::ResponseError),
    /// The caller's own refusal, in its words.
    Vault(String),
    /// The HTTP call to the proxy failed before a command's answer
    /// began.
    CommandRequest(reqwest::Error),
    /// The proxy answered a status other than success: `502` for a
    /// command the caller could not serve or whose answer died before
    /// its first item.
    CommandStatus(u16),
    /// The answer's body failed while it was being read.
    CommandStream(reqwest::Error),
    /// A record of the answer could not be read as a command message.
    CommandAnswer(diverge_provider_sdk::container_proxy::command::response::FrameError),
    /// The answer's body ended without its end record: the command's
    /// answer died mid-stream, and what arrived before is all there
    /// is.
    CommandTruncated,
    /// The caller's own error: the command did not finish. A JSON
    /// value the caller chose, shown here as its JSON — see
    /// [`shared::error::Error`](diverge_provider_sdk::shared::error::Error)
    /// for why it says so little, and why it is a message rather than
    /// a Rust error.
    Command(diverge_provider_sdk::shared::error::Error),
    /// Dialing `/run-loop/agent` failed.
    RunLoopConnect(tokio_tungstenite::tungstenite::Error),
    /// The proxy refused the attachment with a status: `409` for a
    /// harness already attached.
    RunLoopStatus(u16),
    /// The request the proxy handed over did not parse.
    RunLoopRequest(serde_json::Error),
    /// The socket ended before any request came.
    RunLoopClosed,
    /// A chunk or an error would not serialize.
    RunLoopEncode(serde_json::Error),
    /// The loop's socket failed.
    RunLoopSocket(tokio_tungstenite::tungstenite::Error),
    /// The HTTP call to the proxy failed.
    AgentSchemaRequest(reqwest::Error),
    /// The proxy answered a status other than success.
    AgentSchemaStatus(u16),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::McpConnect(error) => {
                write!(f, "connecting to the proxy's MCP server failed: {error}")
            }
            Error::McpService(error) => {
                write!(f, "an MCP exchange with the proxy failed: {error}")
            }
            Error::VaultKey(error) => {
                write!(f, "a vault request could not be written: {error}")
            }
            Error::VaultRequest(error) => {
                write!(f, "a vault request to the proxy failed: {error}")
            }
            Error::VaultStatus(status) => {
                write!(f, "the proxy answered a vault request with {status}")
            }
            Error::VaultAnswer(error) => {
                write!(f, "a vault answer could not be read: {error}")
            }
            Error::Vault(message) => {
                write!(f, "the vault refused: {message}")
            }
            Error::CommandRequest(error) => {
                write!(f, "a command request to the proxy failed: {error}")
            }
            Error::CommandStatus(status) => {
                write!(f, "the proxy answered a command with {status}")
            }
            Error::CommandStream(error) => {
                write!(f, "a command's answer failed mid-stream: {error}")
            }
            Error::CommandAnswer(error) => {
                write!(f, "a command answer could not be read: {error}")
            }
            Error::CommandTruncated => {
                f.write_str("a command's answer ended without its end")
            }
            Error::Command(error) => {
                write!(f, "the command did not finish: {}", error.0)
            }
            Error::RunLoopConnect(error) => {
                write!(f, "attaching to the proxy's loop failed: {error}")
            }
            Error::RunLoopStatus(status) => {
                write!(f, "the proxy refused the loop attachment with {status}")
            }
            Error::RunLoopRequest(error) => {
                write!(f, "the loop's request did not parse: {error}")
            }
            Error::RunLoopClosed => {
                f.write_str("the loop's socket ended before a request came")
            }
            Error::RunLoopEncode(error) => {
                write!(f, "a loop frame did not serialize: {error}")
            }
            Error::RunLoopSocket(error) => {
                write!(f, "the loop's socket failed: {error}")
            }
            Error::AgentSchemaRequest(error) => {
                write!(f, "posting the agent schema failed: {error}")
            }
            Error::AgentSchemaStatus(status) => {
                write!(f, "the proxy refused the agent schema with {status}")
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::McpConnect(error) => Some(error),
            Error::McpService(error) => Some(error),
            Error::VaultKey(error) => Some(error),
            Error::VaultRequest(error) => Some(error),
            Error::VaultAnswer(error) => Some(error),
            Error::CommandRequest(error) => Some(error),
            Error::CommandStream(error) => Some(error),
            Error::CommandAnswer(error) => Some(error),
            Error::RunLoopConnect(error) | Error::RunLoopSocket(error) => Some(error),
            Error::RunLoopRequest(error) | Error::RunLoopEncode(error) => Some(error),
            Error::AgentSchemaRequest(error) => Some(error),
            // A message from the wire, not a Rust error: nothing to
            // chain.
            Error::VaultStatus(_)
            | Error::Vault(_)
            | Error::CommandStatus(_)
            | Error::CommandTruncated
            | Error::Command(_)
            | Error::RunLoopStatus(_)
            | Error::RunLoopClosed
            | Error::AgentSchemaStatus(_) => None,
        }
    }
}
