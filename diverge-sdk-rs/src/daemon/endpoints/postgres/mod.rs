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
//! database, the same for every container. Which database is the
//! daemon's [`Mode`]: LOCAL, a Postgres the daemon runs itself, beside
//! it, with nothing for a client to give; or REMOTE, one the daemon
//! dials at a URL a client gave. [`get`] answers the mode; [`set`]
//! replaces it whole; [`connections`] lists the container connections
//! open through it now.
//!
//! # One database, swapped at rest
//!
//! The daemon serves exactly one database at a time, and swaps it only
//! when nothing is on it. A container connection is a splice of bytes
//! the daemon does not read, so it cannot be moved from one database to
//! another mid-stream: a set while any container connection is open
//! through the current database is answered `InUse`, naming the
//! containers that hold one, and the mode is as it was — whether or not
//! the set would have changed it. A client that wants the swap ends
//! those connections, stopping the containers or waiting them out, and
//! asks again. The mode set, every connection opened after it goes to
//! the new database; nothing stored in the old one is carried over, and
//! the old one is not touched.
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
//! # The URL is given, never answered whole
//!
//! A remote mode's URL may carry a password, and the daemon keeps it; a
//! get answers the URL with the password taken out, as the providers'
//! [credentials](crate::daemon::endpoints::providers) are never
//! answered. A set gives it whole, which is how one rotates.

mod connection;
mod container;
mod mode;

pub use connection::*;
pub use container::*;
pub use mode::*;

pub mod connections;
pub mod get;
pub mod set;
