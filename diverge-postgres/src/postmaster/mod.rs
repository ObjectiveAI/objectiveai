//! The server process: stopped if one is left, started, and ready.
//!
//! [`stop`] ends whatever postmaster `data/postmaster.pid` names, if
//! it runs — fast, then immediately, then by signal — through
//! `pg_ctl`, the one program that can ask a postmaster anything on
//! every platform; a stale pid file is removed. It is what every
//! start does before starting, and what the stop does at the end.
//! [`start`] spawns `postgres` directly on a free loopback port, with
//! the name `diverge-postgres` as its `cluster_name`, and hands back
//! the child and the port; [`ready`] waits until the port accepts, or
//! the child exits. Nothing here leashes the child: it is spawned
//! with nothing that would end it when this program ends, and the
//! next start is what ends it.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod ready;
mod start;
mod stop;

pub use error::*;
pub use ready::*;
pub use start::*;
pub use stop::*;
