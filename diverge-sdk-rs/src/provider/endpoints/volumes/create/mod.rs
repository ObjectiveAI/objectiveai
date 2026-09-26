//! Making a volume.
//!
//! A caller names one and says how big it is; a provider makes it and
//! says so. Split by who SENDS, as everywhere else: the ask is in
//! [`client`], the answer in [`server`].
//!
//! # It outlives the scope that made it
//!
//! Which is what separates this from a
//! [`container run`](crate::provider::endpoints::containers),
//! where the scope IS the container's life and dropping the connection
//! stops it. A volume persists until a
//! [`delete`](super::delete) destroys it — across connections, across
//! restarts, across every container that ever mounted it.
//!
//! That persistence is the whole point. A caller mounts one into a
//! laboratory, the laboratory stops, and the work is still there.

pub mod client;
pub mod server;
