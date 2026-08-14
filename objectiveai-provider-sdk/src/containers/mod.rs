//! Containers — running an image a provider can supply, and joining
//! one that is already running.
//!
//! [`images`](crate::images) settles whether an image is available;
//! [`create`] is what happens once it is; [`connect`] is how anyone
//! else gets in.
//!
//! The two are asymmetric on purpose. A creation owns the container —
//! its scope is the container's life, and it decides who else may
//! attach. A connection owns nothing: it names a container somebody
//! else made and asks to be let in.

pub mod connect;
pub mod create;
