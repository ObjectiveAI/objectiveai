//! The database: the one Postgres the daemon serves into its
//! containers.
//!
//! Something inside a container dials Postgres, and the connection is
//! carried out to the daemon as the provider protocol's
//! [`postgres`](crate::shared::containers::postgres) states, a pair of
//! channels per connection, raw. The daemon is the end that holds the
//! database — the provider's
//! [`PostgresDialer`](crate::provider::client::PostgresDialer) is its
//! part — and it splices every such connection onto exactly one
//! database, the same for every container, and keeps its own records in
//! that database too. Which database is the daemon's [`Mode`], and the
//! mode is the daemon's CONFIGURATION, read when it starts: LOCAL, a
//! Postgres the daemon runs itself, beside it; or REMOTE, one the
//! daemon dials at a URL its configuration gives. [`get`] answers the
//! mode; [`connections`] lists the container connections open through
//! the database now. Nothing on this wire changes the mode: a daemon
//! that is to serve another database is configured and started again,
//! and what the old database holds stays there.
//!
//! # What a container sees
//!
//! Nothing of this. A container's driver dials its own proxy, on the
//! proxy's
//! [`POSTGRES_LOOPBACK_PORT`](crate::container_proxy::inside::POSTGRES_LOOPBACK_PORT),
//! as the one [user](crate::container_proxy::inside::POSTGRES_USER) and
//! [database](crate::container_proxy::inside::POSTGRES_DATABASE) every
//! container names; which Postgres is behind that is the daemon's to
//! decide, and the mode changes nothing the container is told.
//!
//! # The URL is never answered whole
//!
//! A remote mode's URL may carry a password, and the daemon keeps it; a
//! get answers the URL with the password taken out, as the providers'
//! [credentials](crate::daemon::endpoints::providers) are never
//! answered.

mod connection;
mod container;
mod mode;

pub use connection::*;
pub use container::*;
pub use mode::*;

pub mod connections;
pub mod get;
