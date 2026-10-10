//! The mounts the daemon serves into a container, by the id each
//! FUSE ask names.
//!
//! A record's cross-provider mounts are served by a `volumes::serve`
//! scope the daemon holds on the volume's provider for the run's
//! life, every ask forwarded and every answer relayed — the serve is
//! the run's, opened at its start and let go at its end, which is
//! what the idle rule makes of "held for the agent's life". A
//! dependency's mounts are served the same way by a
//! `containers::serve` scope the daemon holds on the agent's provider
//! for the dependency's life: the agent's own path, a directory
//! whole or a file with the directory holding it, into the
//! dependency's container. [`Mounts`] is the set for one run, built
//! before the run opens and asked for its life; [`bridge`] answers
//! every ask through either kind of [`Serve`]. A watch of the
//! container sees the mounts spliced into its tree and every change
//! under one — the ones the daemon makes, for a volume; every one the
//! served container sees, for an agent's path, which the serve
//! streams and the mount's own watch task reroots — which the mounts
//! tell as frames from the container's root.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod changes;
mod mounts;
mod splice;
mod watch;

pub mod bridge;

pub use mounts::*;
