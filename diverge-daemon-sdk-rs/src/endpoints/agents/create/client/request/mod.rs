//! Create request data.
//!
//! What a caller hands the daemon to create an agent under a name:
//! the [`Frame`] itself carries everything the agent is made from —
//! its [`Image`], its limits, its [`FuseMount`]s, its arguments, and
//! the one [`Provider`] it is pinned to with the [`VolumeMount`]s it
//! has there, if any — and the name. The daemon's own
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
