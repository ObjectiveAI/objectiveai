//! The daemon's configuration: where it lives, how it is found, and
//! what it says.
//!
//! - `--config <dir>`, else `DIVERGE_DAEMON_CONFIG`, else
//!   `~/.diverge/daemon/`, names the DIRECTORY, created if absent —
//!   see [`dir`].
//! - `<dir>/config.yaml` is read if present; absent means the built-in
//!   defaults, [`Config::default`]. No other name or extension is
//!   looked for — see [`load`].
//!
//! [`Config`] is the document; [`Error`] is why the directory or the
//! file could not be used.
//!
//! ```yaml
//! port: 14980
//! postgres:
//!   kind: remote
//!   url: postgresql://diverge:secret@db.example.com:5432/diverge
//! idle_seconds: 10
//! ```
//!
//! `postgres` absent is `{kind: local}`: the daemon runs a Postgres of
//! its own, `diverge-postgres`, beside it, under `<dir>/postgres/`.
//! `idle_seconds` is how long a container may go unused before its
//! run is ended; absent, ten.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod config;
mod dir;
mod error;
mod load;

pub use config::*;
pub use dir::*;
pub use error::*;
pub use load::*;
