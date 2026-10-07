//! Why a volume could not be acted on, in the wire's three words.

use std::fmt;

use diverge_sdk::shared::error::Error;

/// What a provider's answer about a volume comes to: no such volume
/// or nothing at the path, the volume held, or a failure in a
/// sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fail {
    /// No volume is the one named, or nothing is at the path.
    NotFound,
    /// A running container has the volume, or an operation is on it.
    Held,
    /// The provider could not be asked, or answered a failure.
    Error(String),
}

impl Fail {
    /// A provider's refusal, read by its `kind`: `volume` and `path`
    /// are nothing to find, `mounted` is held, anything else a
    /// failure in the provider's words.
    pub fn refusal(error: &Error) -> Fail {
        match error.0.get("kind").and_then(|kind| kind.as_str()) {
            Some("volume") | Some("path") => Fail::NotFound,
            Some("mounted") => Fail::Held,
            _ => Fail::Error(describe(error)),
        }
    }

    /// A failure to reach or ask the provider.
    pub fn failed(error: &impl fmt::Debug) -> Fail {
        Fail::Error(format!("{error:?}"))
    }
}

impl fmt::Display for Fail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fail::NotFound => f.write_str("no such volume, or nothing at the path"),
            Fail::Held => f.write_str("the volume is held"),
            Fail::Error(error) => f.write_str(error),
        }
    }
}

/// A provider's error as one sentence: its `error` member when it has
/// one, else the whole value.
pub fn describe(error: &Error) -> String {
    match error.0.get("error").and_then(|text| text.as_str()) {
        Some(text) => text.to_string(),
        None => error.0.to_string(),
    }
}
