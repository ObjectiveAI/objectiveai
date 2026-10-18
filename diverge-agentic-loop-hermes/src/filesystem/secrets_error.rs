//! A fed value that will not do.

use std::error;
use std::fmt;

/// Why the fed values could not be applied to the plan. No variant
/// carries a value: each names the secret, and only the secret.
#[derive(Debug)]
pub enum SecretsError {
    /// The feed holds no value under the name — the proxy fed the
    /// declaration short, or has fed nothing yet.
    Missing(&'static str),
    /// The value is not a JSON string, which an environment variable,
    /// `model.api_key` and the vertex document all are.
    NotString(&'static str),
    /// The value is not a JSON object, which an `auth.json` entry and
    /// the Qwen token file both are.
    NotObject(&'static str),
}

impl fmt::Display for SecretsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecretsError::Missing(name) => {
                write!(f, "the feed holds no value under {name}")
            }
            SecretsError::NotString(name) => {
                write!(f, "the value fed under {name} is not a JSON string")
            }
            SecretsError::NotObject(name) => {
                write!(f, "the value fed under {name} is not a JSON object")
            }
        }
    }
}

impl error::Error for SecretsError {}

impl SecretsError {
    /// The failure as JSON, the shape every failure on the stream has
    /// — kind `secrets`, naming the secret and never its value.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "secrets",
            "error": self.to_string(),
        })
    }
}
