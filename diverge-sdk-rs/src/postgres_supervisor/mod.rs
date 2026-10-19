//! The two lines that cross the stdio of `diverge-postgres`: the
//! ready line out, the shutdown line in.
//!
//! `diverge-postgres` is the Postgres a daemon runs beside itself in
//! its local mode, and the daemon is what starts it. The two speak
//! through the supervisor's own stdio, one JSON object per line each
//! way, and these are the objects — here, in the SDK, so that the
//! supervisor and the daemon name one type each rather than one
//! another's text, and neither depends on the other's crate.
//! [`Ready`] goes out on the supervisor's stdout exactly once, when
//! the postmaster accepts:
//! `{"type":"ready","url":"postgresql://postgres:…@127.0.0.1:…"}`.
//! Nothing else is ever written to that stdout. [`Command`] comes in
//! on its stdin, one per line: `{"type":"shutdown"}` is the one there
//! is, and it stops the postmaster cleanly and ends the supervisor. A
//! line that is not a command is ignored, and so is the end of stdin
//! — a parent that goes away, or a stdin that was never anything, does
//! not stop the cluster; only the line, a signal, or the postmaster
//! itself does.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod command;
mod ready;

pub use command::*;
pub use ready::*;
