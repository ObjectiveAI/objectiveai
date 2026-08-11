//! LEB128, unsigned. Private to the frame codec.

use super::FrameError;

/// Append `value`.
pub(super) fn write(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

/// How many bytes [`write`] will emit for `value`.
pub(super) fn len(mut value: u64) -> usize {
    let mut len = 1;
    while value >= 0x80 {
        value >>= 7;
        len += 1;
    }
    len
}

/// Read one varint, returning it and how many bytes it took.
pub(super) fn read(bytes: &[u8]) -> Result<(u64, usize), FrameError> {
    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        let part = u64::from(byte & 0x7F);
        // Order matters: the shift itself is undefined past 63, so the
        // width check has to short-circuit before the round-trip test.
        if shift >= 64 || (part << shift) >> shift != part {
            return Err(FrameError::Overflow);
        }
        value |= part << shift;
        if byte & 0x80 == 0 {
            return Ok((value, i + 1));
        }
        shift += 7;
    }
    Err(FrameError::Truncated)
}
