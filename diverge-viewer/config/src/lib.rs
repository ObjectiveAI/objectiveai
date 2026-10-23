//! The one `viewer.yaml` the viewer reads.
//!
//! - `--config <dir>`, else `DIVERGE_CONFIG`, else `~/.diverge/`,
//!   names the ROOT, created if absent — see [`root`]. The root is
//!   one directory for every Diverge program on the host, each with
//!   its own document in it and its own directory under it: the
//!   viewer keeps its files under `<root>/viewer/`.
//! - `<root>/viewer.yaml` is read if present; absent means the
//!   built-in defaults, [`Config::default`]. No other name or
//!   extension is looked for — see [`load`].
//! - [`Config`] is the document; [`Daemon`] its `daemon`, which says
//!   which daemon the viewer dials and as whom. A key the document
//!   does not have is refused, so a misspelled setting is refused
//!   rather than ignored.
//!
//! ```yaml
//! daemon:
//!   address: ws://127.0.0.1:14980/
//!   key: root
//! ```
//!
//! [`Error`] is why the root or the file could not be used.
//!
//! A crate of its own, beside the app, so that the CLI reads the
//! document with the viewer's type without depending on the app: the
//! daemon and the provider lend the CLI their types from their own
//! crates, which are libraries; a Tauri app is not.
//!
//! Its own files are flattened into it, so everything is named
//! through this crate and not through the file it lives in.

mod config;
mod daemon;
mod error;
mod load;
mod root;

pub use config::*;
pub use daemon::*;
pub use error::*;
pub use load::*;
pub use root::*;
