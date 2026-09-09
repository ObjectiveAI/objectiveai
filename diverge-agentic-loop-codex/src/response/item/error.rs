//! The `error` item: non-fatal news.

use serde::Deserialize;

/// A non-fatal error surfaced as an item, always completed, under a
/// fresh id: a warning, a config warning (`summary (details)`), a
/// deprecation notice, or a model reroute (`model rerouted: from ->
/// to (reason)`). The run goes on. Named beside the
/// [`Error`](crate::response::Error) EVENT, which is the critical
/// kind.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ErrorItem {
    /// The message.
    pub message: String,
}
