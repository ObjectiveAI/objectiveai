//! `/fuse/mkdir/{channel}`: the shared fuse `mkdir`, re-exported —
//! [`request::Request`] is the ask, [`response::Frame`] the one
//! message that answers it — and, behind the `server` feature, the
//! executor that answers it.

pub use crate::shared::containers::fuse::mkdir::*;

#[cfg(feature = "server")]
pub mod execute;
