//! One frame, as the bytes a payload carries.

use crate::wire::encode::{Encode, Writer};

/// Encode one frame; `None` is a frame that would not serialize,
/// which is nothing to send and nowhere to say so — the channel for
/// saying so is the thing that would not serialize.
pub(crate) fn encoded<T: Encode>(frame: &T) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    frame.encode(&mut Writer::new(&mut out)).ok()?;
    Some(out)
}
