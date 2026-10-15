//! What a provider sends back for an info request.
//!
//! One frame, once: the version and the capabilities — see
//! [`Frame`], and [`Capabilities`] with its
//! [`VolumeCapabilities`] for what the provider holds out.
//!
//! Nothing is aliased here, because nothing here is somebody else's
//! frame.

mod capabilities;
mod frame;
mod volume_capabilities;

pub use capabilities::*;
pub use frame::*;
pub use volume_capabilities::*;
