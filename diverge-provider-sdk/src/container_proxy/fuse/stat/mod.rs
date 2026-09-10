//! `/fuse/stat/{channel}`: the shared fuse `stat`, re-exported —
//! [`request::Request`] is the ask, [`response::Frame`] the one
//! message that answers it — and, behind the `server` feature, the
//! executor that answers it.

pub use crate::shared::containers::fuse::stat::*;

#[cfg(feature = "server")]
pub mod execute;
