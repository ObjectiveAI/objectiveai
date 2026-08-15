//! Laboratories — a container an agent works inside.
//!
//! [`create`] makes one and owns its life. [`connect`] joins one
//! somebody else made, with that somebody's permission.
//!
//! # Why the two are asymmetric
//!
//! Because owning a thing and visiting it are different. A creation's
//! scope IS the container's life: it carries the image pull, mints the
//! id, and ending it stops the container. A connection owns nothing —
//! it names a container it was told about, offers an authorization the
//! provider relays to the creator, and leaving takes nothing with it.
//!
//! What they share, they share through
//! [`shared::container`](crate::shared::container): reading a file,
//! writing one, and moving one between containers are the same
//! exchanges whichever way you got in.

pub mod connect;
pub mod create;
