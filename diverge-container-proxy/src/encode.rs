//! A frame, as the bytes a payload carries.

use diverge_sdk::wire::encode::{Encode, Writer};

/// One frame's bytes; `None` is a frame that would not serialize,
/// which a JSON value never is.
pub fn encoded<F: Encode>(frame: &F) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    frame.encode(&mut Writer::new(&mut out)).ok()?;
    Some(out)
}
