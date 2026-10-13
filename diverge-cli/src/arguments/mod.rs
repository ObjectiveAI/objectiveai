//! The command line: what `diverge` is told.
//!
//! [`Arguments`] is the whole of it — the root and the [`Command`] —
//! parsed by clap, whose help and version are the program's own. The
//! daemon, the account, the request, its body and, for a connect, the
//! [`Mcp`] exchange are [`Run`]'s; which [`Document`] and which
//! [`Action`] on it are [`Config`]'s.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod arguments;
mod command;
mod config;
mod mcp;

pub use arguments::*;
pub use command::*;
pub use config::*;
pub use mcp::*;
