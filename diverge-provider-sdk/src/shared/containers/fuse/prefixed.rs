//! A length-prefixed run of bytes inside a fuse payload: `[len: u16
//! BE][bytes…]`, the one framing every fuse ask uses where something
//! follows.

use super::RequestError;
use crate::encode::Writer;

/// The bytes the prefix occupies.
const LEN: usize = 2;

/// Write `bytes` behind their two-byte length, or say how long they
/// were when they will not fit.
pub(crate) fn put(out: &mut Writer<'_>, bytes: &[u8]) -> Result<(), usize> {
    let len = u16::try_from(bytes.len()).map_err(|_| bytes.len())?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}

/// Split one prefixed run off the front: the run, then the rest.
pub(crate) fn take(bytes: &[u8]) -> Result<(&[u8], &[u8]), RequestError> {
    let len: &[u8; LEN] = bytes
        .get(..LEN)
        .and_then(|head| head.try_into().ok())
        .ok_or(RequestError::Truncated)?;
    let len = usize::from(u16::from_be_bytes(*len));
    let rest = &bytes[LEN..];
    let run = rest.get(..len).ok_or(RequestError::Truncated)?;
    Ok((run, &rest[len..]))
}
