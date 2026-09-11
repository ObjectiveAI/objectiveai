//! `/fuse/read/{channel}`: the shared fuse `read`, re-exported —
//! [`request::Request`] is the ask, [`response::Frame`] the one
//! message that answers it — and, behind the `server` feature, the
//! executor that answers it.

pub use crate::shared::containers::fuse::read::*;

#[cfg(feature = "server")]
pub mod execute;
