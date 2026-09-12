//! `/fuse/list/{channel}`: the shared fuse `list`, re-exported —
//! [`request::Request`] is the ask, [`response::Frame`] the one
//! message that answers it — and, behind the `server` feature, the
//! executor that answers it.

pub use crate::shared::containers::fuse::list::*;

#[cfg(feature = "server")]
pub mod execute;
