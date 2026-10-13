//! The command line: what `diverge` is told.
//!
//! [`Arguments`] is the whole of it — the root, the daemon, the
//! account, and the [`Command`] — parsed by clap, whose help and
//! version are the program's own.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod arguments;
mod command;

pub use arguments::*;
pub use command::*;
