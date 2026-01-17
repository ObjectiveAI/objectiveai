//! Retry token for reusing votes from previous executions.

use base64::Engine;
use serde::{Deserialize, Serialize};

/// Token that enables reusing votes from a previous function execution.
///
/// Contains identifiers for each task's votes that can be reused in a
/// subsequent execution. Serialized as base64-encoded JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RetryToken(pub Vec<Option<String>>);

impl RetryToken {
    /// Appends another token's entries to this token.
    pub fn push(&mut self, other: RetryToken) {
        self.0.extend(other.0)
    }

    /// Inserts another token's entries at a specific index.
    pub fn insert(&mut self, index: usize, other: RetryToken) {
        for (i, token) in other.0.into_iter().enumerate() {
            self.0[index + i] = token;
        }
    }

    /// Serializes the token to a base64-encoded string.
    pub fn to_string(&self) -> String {
        let json = serde_json::to_string(self).unwrap();
        base64::engine::general_purpose::STANDARD.encode(json)
    }

    /// Attempts to deserialize a token from a base64-encoded string.
    pub fn try_from_string(s: &str) -> Option<Self> {
        let json = base64::engine::general_purpose::STANDARD.decode(s).ok()?;
        let token = serde_json::from_slice(&json).ok()?;
        Some(token)
    }
}
