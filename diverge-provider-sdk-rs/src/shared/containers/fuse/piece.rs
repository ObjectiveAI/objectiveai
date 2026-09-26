//! An offset and a length, as a read names its piece.

use super::RequestError;

/// Read `[offset: u64 BE][length: u32 BE]` off the front of `bytes`:
/// the two, and nothing after, since they end a read.
pub(crate) fn decode(bytes: &[u8]) -> Result<(u64, u32), RequestError> {
    let offset: [u8; 8] = bytes.get(..8).and_then(|head| head.try_into().ok()).ok_or(RequestError::Truncated)?;
    let length: [u8; 4] = bytes.get(8..12).and_then(|head| head.try_into().ok()).ok_or(RequestError::Truncated)?;
    Ok((u64::from_be_bytes(offset), u32::from_be_bytes(length)))
}

/// Read `[offset: u64 BE]` off the front: the offset, then the rest.
pub(crate) fn offset(bytes: &[u8]) -> Result<(u64, &[u8]), RequestError> {
    let offset: [u8; 8] = bytes.get(..8).and_then(|head| head.try_into().ok()).ok_or(RequestError::Truncated)?;
    Ok((u64::from_be_bytes(offset), &bytes[8..]))
}
