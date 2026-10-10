//! The one `Daemon` every task borrows.
//!
//! [`Daemon`] is built once at start and shared behind an `Arc` by
//! every connection and every scope: the [`Store`](crate::store::Store)
//! the records are in, and [`Live`], the state that is nobody's record
//! — which accounts have a client connected now, counted so that two
//! connections as one account are two, which providers and daemons
//! are connected, what runs, which exposures are open. A connection
//! admitted counts itself in, and counts itself out when it ends.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod changes;
mod daemon;
mod live;
mod peers;

pub use changes::*;
pub use daemon::*;
pub use live::*;
pub use peers::*;
