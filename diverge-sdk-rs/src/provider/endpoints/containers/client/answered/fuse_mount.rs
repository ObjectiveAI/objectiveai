//! Whether a mount is made.

use std::convert::Infallible;

use bytes::Bytes;

use super::Answered;
use crate::wire::decode::Decode as _;
use crate::shared::containers::fuse_mount;

/// The provider's one answer to a mount asked for while the container
/// runs, whole — made, or why not, the proxy's error included. The
/// frame is the answer, `Error` included: nothing refuses the ask
/// apart from what it says.
#[derive(Debug, Clone, Copy)]
pub struct FuseMount;

impl Answered for FuseMount {
    type Item = fuse_mount::response::Frame;
    type Error = fuse_mount::response::FrameError;
    type Refusal = Infallible;

    fn decode(payload: Bytes) -> Result<Result<Self::Item, Infallible>, Self::Error> {
        fuse_mount::response::Frame::decode(&payload).map(Ok)
    }
}
