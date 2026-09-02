//! The failure, whole.

use serde::{Deserialize, Serialize};

/// The body of one `POST /continuation/error`.
///
/// JSON where the delivery itself is binary, because an error is a
/// document, not bytes. The one field carries the server's full
/// error verbatim — whatever vocabulary the server has for why the
/// delivery can never complete — and the container hands it on,
/// uninterpreted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// The full error, the server's own words.
    pub error: serde_json::Value,
}
