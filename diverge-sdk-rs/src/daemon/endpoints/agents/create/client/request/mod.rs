//! Create request data.
//!
//! What a caller hands the daemon to create an agent under a name, in
//! one [`Inner`](crate::daemon::create::Inner) shared with the tools
//! create and flattened into the [`Frame`]:
//! the [`Frame`] names the
//! [`template`](crate::daemon::endpoints::agents::templates) the
//! agent is made from — its image, its limits, its arguments — and carries what is the agent's own: the one
//! [`Provider`] it is pinned to with the [`VolumeMount`]s it has
//! there, if any, its [`FuseMount`]s from whichever providers hold
//! them, and the name. [`Image`] is the shape a template and a tool
//! name their image by, defined here beside the mounts. The daemon's own
//! definitions, not the provider's container request: what a daemon
//! asks a provider for on an agent's behalf is the daemon's to
//! compose, and the two will part.

mod frame;
mod fuse_mount;
mod image;
mod provider;
mod volume_mount;

pub use frame::*;
pub use fuse_mount::*;
pub use image::*;
pub use provider::*;
pub use volume_mount::*;
