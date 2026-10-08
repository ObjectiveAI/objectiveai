//! The `daemon` block of `config.yaml`: what `diverge-daemon` is
//! told, and what `diverge-postgres` takes from it.
//!
//! [`Config`] is the block; [`Postgres`] its `postgres`, which the
//! daemon reads for which database it runs on and the supervisor
//! reads for the cluster it runs. The daemon keeps its state under
//! `<root>/daemon/`: `agents/`, and `postgres/`, the local cluster's
//! own directory.
//!
//! ```yaml
//! daemon:
//!   port: 14980
//!   postgres:
//!     kind: local
//!     max_connections: 1024
//!   idle_seconds: 10
//! ```
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod config;
mod postgres;

pub use config::*;
pub use postgres::*;
