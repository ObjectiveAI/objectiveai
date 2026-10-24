//! The one `issuer.yaml` the issuer reads.
//!
//! - `--config <dir>`, else `DIVERGE_CONFIG`, else `~/.diverge/`,
//!   names the ROOT, created if absent — see [`root`]. The root is
//!   one directory for every Diverge program on the host, each with
//!   its own document in it: the issuer keeps its database,
//!   `issuer.sqlite`, in the root.
//! - `<root>/issuer.yaml` is read if present; absent means the
//!   built-in defaults, [`Config::default`]. No other name or
//!   extension is looked for — see [`load`].
//! - A key the document does not have is refused, so a misspelled
//!   setting is refused rather than ignored.
//!
//! ```yaml
//! port: 14981
//! issuer: https://auth.example.com
//! access_token_seconds: 600
//! refresh_token_days: 30
//! device_code_seconds: 600
//! device_interval_seconds: 5
//! ```
//!
//! No secret is in the document: the signing keys are in the issuer's
//! database, and every other secret the issuer knows is a hash
//! there.
//!
//! [`Error`] is why the root or the file could not be used.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod config;
mod error;
mod load;
mod root;

pub use config::*;
pub use error::*;
pub use load::*;
pub use root::*;
