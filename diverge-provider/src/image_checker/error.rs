//! Why a check did not answer: it always does.

use std::fmt;

use diverge_provider_sdk::shared::error;

/// What [`ImageChecker`](super::ImageChecker) fails with: nothing. A
/// lookup in a list has no way to fail, and the SDK asks for an
/// error type all the same, so this is one with no value. Every
/// `match` on it is empty, and none is ever reached.
#[derive(Debug)]
pub enum Error {}

impl fmt::Display for Error {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {}
    }
}

/// What the SDK would put on the wire for one of these, which is
/// nothing, since there is none.
impl From<Error> for error::Error {
    fn from(error: Error) -> Self {
        match error {}
    }
}
