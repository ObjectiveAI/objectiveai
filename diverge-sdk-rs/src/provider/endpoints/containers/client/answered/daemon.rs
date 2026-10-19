//! What the proxy answers the provider's half of a daemon connection
//! with.

use std::convert::Infallible;

use bytes::Bytes;

use super::Answered;
use crate::wire::decode::Decode as _;
use crate::shared::containers::daemon;

/// One client frame the program sent on the connection, owned: what
/// the provider's half of the pair carries, in the order the program
/// sent them.
#[derive(Debug, Clone, Copy)]
pub struct Daemon;

impl Answered for Daemon {
    type Item = daemon::client::Owned;
    type Error = daemon::client::FrameError;
    type Refusal = Infallible;

    fn decode(payload: Bytes) -> Result<Result<daemon::client::Owned, Infallible>, daemon::client::FrameError> {
        let frame = daemon::client::Frame::decode(&payload)?;
        Ok(Ok(daemon::client::Owned::from(frame)))
    }
}
