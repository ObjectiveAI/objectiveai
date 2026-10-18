//! What the arguments may be: the image's own word.

use bytes::Bytes;

use super::Answered;
use crate::wire::decode::Decode as _;
use crate::shared::containers::info;
use crate::shared::error::Error;

/// The image's `/DIVERGE.md`, parsed, once — `None` when the image
/// ships none, which is a fact about the image and not a refusal —
/// or the provider's refusal: the file could not be read, or is not a
/// document.
#[derive(Debug, Clone, Copy)]
pub struct Info;

impl Answered for Info {
    type Item = Option<info::Document>;
    type Error = info::response::FrameError;
    type Refusal = Error;

    fn decode(payload: Bytes) -> Result<Result<Option<info::Document>, Error>, Self::Error> {
        Ok(match info::response::Frame::decode(&payload)? {
            info::response::Frame::Info(document) => Ok(Some(document)),
            info::response::Frame::Missing => Ok(None),
            info::response::Frame::Error(error) => Err(error),
        })
    }
}
