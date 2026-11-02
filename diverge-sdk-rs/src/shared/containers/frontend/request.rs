//! What opens a frontend request: its method, its path and its headers.

use serde::{Deserialize, Serialize};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The prefix every reachable path begins with.
pub const PREFIX: &str = "/frontend";

/// One HTTP request's head, as JSON: the method, the path with its
/// query, and the headers in their order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[derive(schemars::JsonSchema)]
pub struct Head {
    /// The method, as HTTP writes it: `GET`, `POST`, ….
    pub method: String,
    /// The path and the query, as the request line carries them:
    /// `/frontend/items?page=2`. See [`reachable`](Self::reachable).
    pub path: String,
    /// The headers, each a name and a value, in their order; a name may
    /// repeat.
    #[serde(default)]
    pub headers: Vec<(String, String)>,
}

impl Head {
    /// Whether the path is one a frontend request may reach.
    ///
    /// The rules:
    ///
    /// 1. It is [`PREFIX`] itself, or begins with [`PREFIX`] followed
    ///    by `/` or `?`.
    /// 2. No segment of it before the query is `.` or `..`.
    /// 3. It holds no `\`, and no `%2e`, `%2f` or `%5c`, in either case.
    pub fn reachable(&self) -> bool {
        let path = self.path.as_str();
        // Rule 1.
        let Some(after) = path.strip_prefix(PREFIX) else {
            return false;
        };
        if !(after.is_empty() || after.starts_with('/') || after.starts_with('?')) {
            return false;
        }
        // Rule 3.
        let lower = path.to_ascii_lowercase();
        if lower.contains('\\') || lower.contains("%2e") || lower.contains("%2f") || lower.contains("%5c") {
            return false;
        }
        // Rule 2.
        let before = path.split('?').next().unwrap_or(path);
        !before.split('/').any(|segment| segment == "." || segment == "..")
    }
}

impl Encode for Head {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Head {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}
