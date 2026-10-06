//! The answer: the server frames that answer a client frame, or the
//! error that says none will.
//!
//! [`Frame`] is what one channel response holds — one [`Served`]
//! frame, or the error; [`Served`] is a server frame of the daemon
//! connection as it crosses a wire; [`Owned`] is the same frame held,
//! for the caller's [`Daemon`](crate::provider::client::Daemon) to
//! hand back.

mod error;
mod frame;
mod owned;
mod served;

pub use error::*;
pub use frame::*;
pub use owned::*;
pub use served::*;
