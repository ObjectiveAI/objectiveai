//! The three answers a scope that carries bytes shares with every
//! other: no such thing, forbidden, or the daemon's own error.

use diverge_sdk::shared::error::Error;
use serde::Serialize;

/// An answer that is not the exchange's result: printed as the
/// daemon's own frame would be — `"not_found"`, `"forbidden"`,
/// `{"error":…}` — by the shapes that print lines of their own beside
/// it, the download and the connect.
#[derive(Debug, Clone, PartialEq, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    /// No such thing is the one named.
    NotFound,
    /// The account holds no grant allowing it.
    Forbidden,
    /// The daemon's own failure.
    Error(Error),
}
