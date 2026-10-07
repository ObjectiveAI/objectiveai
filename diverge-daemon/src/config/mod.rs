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
//! ```
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
