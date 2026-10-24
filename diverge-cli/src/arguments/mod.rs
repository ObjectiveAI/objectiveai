//! The command line: what `diverge` is told.
//!
//! [`Arguments`] is the whole of it — the root and the [`Command`] —
//! parsed by clap, whose help and version are the program's own. The
//! `run` command's own tree — and `request-schema` and
//! `response-schema` beneath each of
//! its requests — is [`run`](crate::run)'s; which
//! [`Document`] and which [`Action`] on it are [`Config`]'s; whether
//! the daemon or the provider is started or stopped is
//! [`Lifecycle`]; the issuer's own commands are [`Issuer`].
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod arguments;
mod command;
mod config;
mod issuer;
mod keyring;
mod lifecycle;
mod link;

pub use arguments::*;
pub use command::*;
pub use config::*;
pub use issuer::*;
pub use keyring::*;
pub use lifecycle::*;
pub use link::*;
