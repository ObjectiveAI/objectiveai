//! A moment, as a stat carries one and a setattr sets one.

use super::ResponseError;
use crate::wire::encode::Writer;

/// Seconds since the Unix epoch and nanoseconds into the second:
/// twelve bytes, fixed.
///
/// ```text
/// [secs: u64 BE][nanos: u32 BE]
/// ```
///
/// Unsigned, so nothing predates 1970; nanoseconds, because a build
/// tool that compares modification times compares them at the
/// filesystem's own resolution, and a second would make two saves
/// one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Time {
    /// Seconds since `1970-01-01T00:00:00Z`.
    pub secs: u64,
    /// Nanoseconds into the second, under a billion.
    pub nanos: u32,
}

/// The bytes a time occupies.
pub(crate) const TIME_LEN: usize = 8 + 4;

impl Time {
    /// Write the twelve bytes.
    pub(crate) fn encode(&self, out: &mut Writer<'_>) {
        out.extend_from_slice(&self.secs.to_be_bytes());
        out.extend_from_slice(&self.nanos.to_be_bytes());
    }

    /// Read the twelve bytes off the front: the time, then the rest.
    pub(crate) fn decode(bytes: &[u8]) -> Result<(Self, &[u8]), ResponseError> {
        let fixed = bytes.get(..TIME_LEN).ok_or(ResponseError::Truncated)?;
        let secs: [u8; 8] = fixed[..8].try_into().expect("eight bytes were taken");
        let nanos: [u8; 4] = fixed[8..].try_into().expect("four bytes were taken");
        Ok((
            Time {
                secs: u64::from_be_bytes(secs),
                nanos: u32::from_be_bytes(nanos),
            },
            &bytes[TIME_LEN..],
        ))
    }
}
