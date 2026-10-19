//! What the caller may still use, counted on this host.
//!
//! [`CapacityReporter`] answers the four numbers of the
//! [`capacity`](diverge_sdk::provider::endpoints::capacity) endpoint
//! from the pieces that hold them: the volumes' stores, the client
//! image store and the deployer's two caps. This host partitions
//! nobody — every number is the whole of the room, which is what any
//! one caller may use — so the caller's identity narrows nothing
//! here.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod capacity_reporter;
mod error;

pub use capacity_reporter::*;
pub use error::*;
