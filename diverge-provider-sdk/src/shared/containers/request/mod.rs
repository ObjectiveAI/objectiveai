//! What every request for a container has to say, whatever kind it is.
//!
//! [`Container`] is the whole of asking for one: an [`Image`], the
//! limits, and the mounts — a [`VolumeMount`] for a volume the
//! provider offers, a [`IdentityMount`] for content the caller holds.
//! Every run in [`containers`](crate::endpoints::containers) sends one
//! of these and nothing more; what differs between the kinds is asked
//! later, on a channel, not here. [`Connect`] is the other way to get
//! a container: naming one somebody else runs, with whatever its
//! runner needs to say yes.
//!
//! They live here rather than in either family because
//! [`ContainerDeployer`](crate::server::container_deployer::ContainerDeployer)
//! serves both, and a generic deployer naming one family's type would
//! be the thing this crate spends its exceptions avoiding.

mod connect;
mod container;
mod identity_mount;
mod image;
mod volume_mount;

pub use connect::*;
pub use container::*;
pub use identity_mount::*;
pub use image::*;
pub use volume_mount::*;
