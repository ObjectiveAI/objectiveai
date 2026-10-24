//! The one `daemon.yaml` the daemon reads.
//!
//! - `--config <dir>`, else `DIVERGE_CONFIG`, else `~/.diverge/`,
//!   names the ROOT, created if absent — see [`root`]. The root is
//!   one directory for every Diverge program on the host, each with
//!   its own document in it and its own directory under it: the
//!   daemon keeps its state under `<root>/daemon/`, and its Postgres
//!   under `<root>/daemon/postgres/`.
//! - `<root>/daemon.yaml` is read if present; absent means the
//!   built-in defaults, [`Config::default`]. No other name or
//!   extension is looked for — see [`load`].
//! - [`Config`] is the document; [`Postgres`] its `postgres`, which
//!   says which database the daemon runs on and, for its own, what
//!   the cluster is given — handed to `diverge-postgres` as arguments
//!   when the daemon starts it, which reads no document. A key the
//!   document does not have is refused, so a misspelled setting is
//!   refused rather than ignored.
//!
//! ```yaml
//! port: 14980
//! identities:
//!   https://auth.example.com: daemon-1
//! providers:
//!   accepted:
//!     - issuer: https://auth.example.com
//!       subject: provider-1
//!   connect:
//!     - address: provider.example.com:14979
//!       issuer: https://auth.example.com
//! daemons:
//!   accepted:
//!     - issuer: https://auth.example.com
//!       subject: daemon-2
//!   connect:
//!     - name: lab
//!       issuer: https://auth.example.com
//!       links:
//!         - provider: { issuer: https://auth.example.com, subject: provider-1 }
//!           daemon: { issuer: https://auth.example.com, subject: daemon-2 }
//! postgres:
//!   kind: local
//!   max_connections: 1024
//! idle_seconds: 10
//! accept_daemons: true
//! capacity_poll_seconds: 10
//! ```
//!
//! [`Error`] is why the root or the file could not be used.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod config;
mod daemons;
mod error;
mod load;
mod postgres;
mod providers;
mod root;

pub use config::*;
pub use daemons::*;
pub use error::*;
pub use load::*;
pub use postgres::*;
pub use providers::*;
pub use root::*;
