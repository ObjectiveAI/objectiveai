//! The program, running: the configuration resolved, the command
//! done.
//!
//! [`run`] is the whole of it — [`config`] the part of the `config`
//! beneath every program,
//! part, [`lifecycle`](crate::lifecycle) the `daemon` and `provider`
//! commands' — and [`Error`] why it could not be. A
//! run prints nothing of its own but the one line a start or a stop
//! says: the one report a failed run gets is the error `main`
//! returns.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod run;

pub mod config;
pub mod issuer;
pub mod keyring;
pub mod link;

pub use error::*;
pub use run::*;
