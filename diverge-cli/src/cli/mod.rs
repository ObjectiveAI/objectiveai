//! The program, running: the configuration resolved, the command
//! done.
//!
//! [`run`] is the whole of it — [`config`] the `config` command's
//! part — and [`Error`] why it could not be. A
//! run prints nothing of its own: the one report a failed run gets is
//! the error `main` returns.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod run;

pub mod config;

pub use error::*;
pub use run::*;
