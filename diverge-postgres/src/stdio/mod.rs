//! The two lines that cross stdio: the ready line out, the shutdown
//! line in.
//!
//! This program speaks to whatever started it through its own stdio,
//! one JSON object per line each way, and these are the objects, in
//! the library so that the daemon names them rather than their text.
//! [`Ready`] goes out on stdout exactly once, when the postmaster
//! accepts: `{"type":"ready","url":"postgresql://postgres:…@127.0.0.1:…"}`.
//! Nothing else is ever written to stdout. [`Command`] comes in on
//! stdin, one per line: `{"type":"shutdown"}` is the one there is,
//! and it stops the postmaster cleanly and ends the program. A line
//! that is not a command is ignored, and so is the end of stdin — a
//! parent that goes away, or a stdin that was never anything, does
//! not stop the cluster; only the line, a signal, or the postmaster
//! itself does.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod command;
mod ready;

pub use command::*;
pub use ready::*;
