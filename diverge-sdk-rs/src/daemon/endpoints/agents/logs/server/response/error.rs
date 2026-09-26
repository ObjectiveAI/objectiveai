//! An error a run answered with, as the log keeps it.

use crate::shared;
use serde::{Deserialize, Serialize};

/// One error the log kept: `type` is the string `error`, and the
/// error itself — the run that would not start, or the message the
/// agent refused, in the words the daemon received — is whole under
/// `error`.
///
/// The error is an arbitrary JSON value, as
/// [`shared::error::Error`] says; it is kept under its own member
/// rather than flattened so that nothing in it can be mistaken for
/// the item's own `type`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Error {
    /// Always `error`.
    pub r#type: ErrorType,
    /// The error, whole.
    pub error: shared::error::Error,
}

/// [`Error`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ErrorType {
    #[serde(rename = "error")]
    #[default]
    Error,
}
