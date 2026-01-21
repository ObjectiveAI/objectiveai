//! API key types and definitions.

use crate::prefixed_uuid::PrefixedUuid;
use serde::{Deserialize, Serialize};

/// An ObjectiveAI API Key.
///
/// The format is always `apk` followed by 32 hexadecimal characters
/// representing a UUID (e.g., `apk1234567890abcdef1234567890abcdef`).
///
/// API keys are used to authenticate requests to the ObjectiveAI API.
pub type ApiKey = PrefixedUuid<'a', 'p', 'k'>;

/// An ObjectiveAI API Key with associated metadata.
///
/// This struct contains the API key itself along with information about
/// when it was created, when it expires (if ever), whether it has been
/// disabled, and user-provided name and description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyWithMetadata {
    /// The API key itself.
    pub api_key: ApiKey,
    /// The timestamp when the API key was created (RFC 3339 format).
    pub created: chrono::DateTime<chrono::Utc>,
    /// The timestamp when the API key expires, or `None` if it does not expire.
    pub expires: Option<chrono::DateTime<chrono::Utc>>,
    /// The timestamp when the API key was disabled, or `None` if it is active.
    pub disabled: Option<chrono::DateTime<chrono::Utc>>,
    /// The user-provided name of the API key.
    pub name: String,
    /// The user-provided description of the API key, or `None` if not provided.
    pub description: Option<String>,
}
