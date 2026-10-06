//! The program, running: the start in order, the wait, and the stop.
//!
//! [`run`] is the whole: the binaries extracted if need be, the init
//! lock taken, the password read or minted, the cluster initialized
//! if need be, the postmaster the last start left stopped, a fresh
//! one started and waited on until it accepts, the lock let go, the
//! ready line written, and then the wait — for a shutdown line on
//! stdin, for Ctrl-C or SIGTERM, or for the postmaster to exit on its
//! own — and the stop. [`Error`] is why it could not start, or why it
//! ended other than by being told to: the one report there is, so
//! its `Debug` is its `Display`. Nothing else is printed.
//!
//! # The init lock
//!
//! `bin/locks/init.lock` is held from before the cluster is looked at
//! until the postmaster accepts, so two starts on one directory take
//! turns at the whole of it and the second finds the first's
//! postmaster running and stops it. It is serialization, not
//! readiness: readiness is the line on stdout.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod run;

pub use error::*;
pub use run::*;
