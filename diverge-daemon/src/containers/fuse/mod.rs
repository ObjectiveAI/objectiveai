//! The mounts the daemon serves into a container, by the id each
//! FUSE ask names.
//!
//! A template's resource mounts are served from the daemon's own
//! content: read-only, in place; ephemeral, from a copy made for the
//! run under `<dir>/overlays/` and removed with it. A create's
//! cross-provider mounts are served by a `volumes::serve` scope the
//! daemon holds on the volume's provider for the run's life, every
//! ask forwarded and every answer relayed — the serve is the run's,
//! opened at its start and let go at its end, which is what the idle
//! rule makes of "held for the agent's life". [`Mounts`] is the set
//! for one run, built before the run opens and asked for its life;
//! [`resources`] answers the first kind, [`bridge`] the second.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod mounts;

pub mod bridge;
pub mod resources;

pub use mounts::*;
