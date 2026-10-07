//! What goes out on the supervisor's stdout.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

/// The one line the supervisor's stdout ever carries, written when
/// the postmaster accepts: `{"type":"ready","url":"…"}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename = "ready")]
pub struct Ready {
    /// Where the cluster is and how to reach it as its superuser:
    /// `postgresql://postgres:<password>@127.0.0.1:<port>`, the
    /// password percent-encoded, and no database named — the daemon
    /// names its own.
    pub url: String,
}

impl Ready {
    /// The line for a cluster whose superuser password is `password`,
    /// on `port`.
    pub fn new(password: &str, port: u16) -> Self {
        Ready {
            url: format!("postgresql://postgres:{}@127.0.0.1:{port}", percent_encode(password)),
        }
    }
}

/// `password` as a URL's userinfo may carry it: unreserved bytes as
/// they are, every other byte as `%XX`.
fn percent_encode(password: &str) -> String {
    let mut encoded = String::with_capacity(password.len());
    for byte in password.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(byte as char);
        } else {
            // Writing to a String cannot fail.
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}
