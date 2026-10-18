//! What every request for a container has to say, whatever kind it is.
//!
//! [`Container`] is the whole of asking for one: an [`Image`] — a
//! digest, and the [`Reference`]s under which its bytes may be fetched
//! — the limits, the mounts — a [`VolumeMount`] for a volume the provider
//! offers, a [`FuseMount`] for what the caller serves live — and the
//! arguments the image is handed once. Every run in
//! [`containers`](crate::provider::endpoints::containers) sends one of these
//! and nothing more; what differs between the kinds is said later,
//! on a channel, not here.
//!
//! They live here rather than in either family because
//! `ContainerDeployer`
//! serves both, and a generic deployer naming one family's type would
//! be the thing this crate spends its exceptions avoiding.

mod container;
mod fuse_mount;
mod image;
mod reference;
mod volume_mount;

pub use container::*;
pub use fuse_mount::*;
pub use image::*;
pub use reference::*;
pub use volume_mount::*;
