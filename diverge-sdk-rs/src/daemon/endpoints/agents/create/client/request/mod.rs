//! Create request data.
//!
//! What a caller hands the daemon to create an agent under a name:
//! the [`Frame`] names the
//! [`template`](crate::daemon::endpoints::agents::templates) the
//! agent is made from — its image, its limits, its provider pin, its
//! arguments — and carries what is the agent's own: the
//! [`VolumeMount`]s of the provider the template pins it to, its
//! [`FuseMount`]s from whichever providers hold them, and the name.
//! [`Image`] and [`Provider`] are the shapes a template and a tool
//! are made of, defined here beside the mounts. The daemon's own
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
