//! The database the daemon runs on: started beside it, or dialled.
//!
//! The daemon keeps its records in, and serves its containers from,
//! one Postgres, and its configuration says which — see
//! [`Postgres`](diverge_sdk::config::daemon::Postgres). In the
//! LOCAL mode [`spawn`] starts the `diverge-postgres` program found
//! beside the daemon's own executable, on the same root — it reads
//! the same `config.yaml`, and keeps its cluster under
//! `<dir>/postgres/` — and reads the one line it writes, the URL of the cluster
//! it brought up; at the daemon's stop it writes that program the one
//! line it reads, and waits for it to end. In the REMOTE mode the URL
//! is the configuration's, as given. [`start`] is the one call that
//! does either and answers the URL the store opens; [`Postgres`] is
//! what is held until the stop.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod local;
mod postgres;

pub use error::*;
pub use local::*;
pub use postgres::*;
