//! Why a password could not be hashed.

use std::fmt;

/// Why a password could not be hashed.
#[derive(Debug)]
pub enum Error {
    /// argon2 refused.
    Hash(argon2::password_hash::Error),
    /// The blocking task the hash ran on panicked.
    Panicked,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Hash(error) => write!(f, "the password could not be hashed: {error}"),
            Error::Panicked => f.write_str("the password's hash panicked"),
        }
    }
}

impl std::error::Error for Error {}
