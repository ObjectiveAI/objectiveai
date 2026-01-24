use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RetryToken(pub Vec<Option<String>>);

impl RetryToken {
    pub fn push(&mut self, other: RetryToken) {
        self.0.extend(other.0)
    }

    pub fn insert(&mut self, index: usize, other: RetryToken) {
        for (i, token) in other.0.into_iter().enumerate() {
            self.0[index + i] = token;
        }
    }

    pub fn to_string(&self) -> String {
        let json = serde_json::to_string(self).unwrap();
        base64::engine::general_purpose::STANDARD.encode(json)
    }

    pub fn try_from_string(s: &str) -> Option<Self> {
        let json = base64::engine::general_purpose::STANDARD.decode(s).ok()?;
        let token = serde_json::from_slice(&json).ok()?;
        Some(token)
    }
}
