//! What every session's client shares: where the server is, and how
//! a client's failures are told.

use diverge_sdk::container_proxy::inside::port;
use rmcp::{ErrorData, ServiceError};

/// The server's URL, for every path.
pub fn url() -> String {
    format!("http://127.0.0.1:{}/mcp", port())
}

/// A call that never reached the server, or that the transport lost,
/// in the exchange's own vocabulary.
pub fn refused(reason: String) -> ErrorData {
    ErrorData::internal_error(reason, None)
}

/// What a call came to: the server's own error travels as itself,
/// code and all; the client's failures are internal errors.
pub fn outcome<T>(result: Result<T, ServiceError>) -> Result<T, ErrorData> {
    match result {
        Ok(value) => Ok(value),
        Err(ServiceError::McpError(error)) => Err(error),
        Err(error) => Err(refused(error.to_string())),
    }
}
