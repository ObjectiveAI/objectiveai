//! The command line: what `diverge` is told.
//!
//! [`Arguments`] is the whole of it — the root and the [`Command`] —
//! parsed by clap, whose help and version are the program's own. The
//! `run` command's own tree — and `request-schema` and
//! `response-schema` beneath each of
//! its requests — is [`run`](crate::run)'s. Every program's document
//! is worked on beneath the program, `diverge <program> config`, by an
//! [`Action`]: the daemon and the provider are each a [`Server`] —
//! started or stopped by a [`Lifecycle`], or configured — the viewer
//! and the CLI each a [`Document`], configured alone; the issuer's own
//! commands, its lifecycle and its document among them, are
//! [`Issuer`].
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod arguments;
mod command;
mod config;
mod document;
mod issuer;
mod keyring;
mod lifecycle;
mod link;
mod server;

pub use arguments::*;
pub use command::*;
pub use config::*;
pub use document::*;
pub use issuer::*;
pub use keyring::*;
pub use lifecycle::*;
pub use link::*;
pub use server::*;
