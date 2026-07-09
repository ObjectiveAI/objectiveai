//! The podman layer's error type: a message describing which podman
//! operation failed. Self-contained so consumers (CLI, viewer) map it
//! into their own error enums however they like.

/// A failed podman operation, described.
#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "podman: {}", self.0)
    }
}

impl std::error::Error for Error {}
