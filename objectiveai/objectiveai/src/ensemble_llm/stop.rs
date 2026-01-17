//! Stop sequence configuration for Ensemble LLMs.

use serde::{Deserialize, Serialize};

/// Stop sequences that terminate model generation.
///
/// When the model generates any of these sequences, it immediately
/// stops producing further tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Stop {
    /// A single stop sequence.
    String(String),
    /// Multiple stop sequences (up to 4 typically supported).
    Strings(Vec<String>),
}

impl Stop {
    /// Normalizes the stop configuration for deterministic hashing.
    ///
    /// - Empty arrays become `None`
    /// - Single-element arrays become `String` variant
    /// - Multi-element arrays are sorted and deduplicated
    pub fn prepare(self) -> Option<Self> {
        if let Stop::Strings(mut strings) = self {
            if strings.is_empty() {
                None
            } else if strings.len() == 1 {
                Some(Stop::String(strings.into_iter().next().unwrap()))
            } else {
                strings.sort();
                strings.dedup();
                Some(Stop::Strings(strings))
            }
        } else {
            Some(self)
        }
    }

    /// Validates that stop sequences are non-empty strings.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Stop::String(string) if string.is_empty() => {
                Err("`stop` string cannot be empty".to_string())
            }
            Stop::Strings(strings) if strings.iter().any(|s| s.is_empty()) => {
                Err("`stop` strings cannot be empty".to_string())
            }
            _ => Ok(()),
        }
    }
}
