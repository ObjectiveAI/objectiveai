//! Why a volume operation failed.

use std::fmt;

/// What [`Volumes`](super::Volumes) fails with. No variant yet: the
/// failures arrive with the implementation.
#[derive(Debug)]
pub enum Error {}

impl fmt::Display for Error {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl std::error::Error for Error {}
