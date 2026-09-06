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
            Error::VaultStatus(_) | Error::Vault(_) => None,
        }
    }
}
