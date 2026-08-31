//! The container taking one delivery.

use serde::{Deserialize, Serialize};

/// The 2xx body of one `POST /resource`, chunk and completion
/// alike.
///
/// It says only that the POST was taken — the chunk appended, or
/// the completion noted. There is nothing else a success could say:
/// whether the assembled resource is RIGHT is the identity's
/// promise, and a failure to take a delivery at all is HTTP's to
/// report, per the surface's contract.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Response {
    /// Always `received`.
    pub r#type: ReceivedType,
}

/// The `received` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ReceivedType {
    /// The only value.
    #[default]
    Received,
}
