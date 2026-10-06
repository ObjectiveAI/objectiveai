//! Where a transfer lands.
//!
//! A transfer — an
//! [agent's](crate::daemon::endpoints::agents::transfer), a
//! [tool's](crate::daemon::endpoints::tools::transfer), a
//! [resource's](crate::daemon::endpoints::resources::transfer) — copies
//! a file or a directory out of its source into a [`Destination`]: a
//! path in an agent's container, a path in a tool's, or a new resource,
//! and the bytes never reach the client. The destination is one shape
//! for the three, defined here.

mod destination;
mod resource;

pub use destination::*;
pub use resource::*;
