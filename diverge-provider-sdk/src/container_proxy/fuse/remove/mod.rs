//! `/fuse/remove/{channel}`: the shared fuse `remove`, re-exported —
//! [`request::Request`] is the ask, [`response::Frame`] the one
//! message that answers it — and, behind the `server` feature, the
//! executor that answers it.

pub use crate::shared::containers::fuse::remove::*;

#[cfg(feature = "server")]
pub mod execute;
