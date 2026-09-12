//! Postgres: where the driver dials.
//!
//! The proxy is a database to the program beside it — a pgwire
//! listener on the container's loopback that carries every connection
//! to the caller's own database, raw, one connection one session. No
//! method here speaks the protocol: the program's driver does, as it
//! would to any Postgres, and what it needs from this crate is where
//! — the address, or the whole URL with the one identity every
//! container uses.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use diverge_provider_sdk::container_proxy::postgres;

use crate::Client;

/// The user every container connects as. Not a credential: the
/// caller's side is the authority on what the connection may reach,
/// and passes the name through unread.
pub const POSTGRES_USER: &str = "diverge";

/// The database every container names, for the same reason.
pub const POSTGRES_DATABASE: &str = "diverge";

impl Client {
    /// Where to point the driver: the proxy's pgwire listener on the
    /// container's loopback.
    ///
    /// Every connection the driver opens here is one session with the
    /// caller's database, opened as the driver asks and held until
    /// either side hangs up; a pool is the ordinary case. The user,
    /// the database and the password are whatever the caller's
    /// database wants — the proxy passes them through unread, as it
    /// passes everything, TLS negotiation included. A session that
    /// ended is a hang-up the driver already knows how to report, and
    /// the proxy retries nothing: a socket cannot be resumed, and a
    /// pool reconnects on its own.
    pub fn postgres_address(&self) -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::LOCALHOST,
            postgres::LOOPBACK_PORT,
        ))
    }

    /// The whole URL, for a driver that takes one:
    /// `postgres://diverge@127.0.0.1:81/diverge?sslmode=disable`.
    ///
    /// The user and the database are [`POSTGRES_USER`] and
    /// [`POSTGRES_DATABASE`], fixed, with no password — the caller's
    /// side decides what they mean, and a container has no credential
    /// of its own to offer. TLS is off, because the proxy is loopback
    /// and the conduit beyond it is the provider's to secure; a driver
    /// that negotiated TLS here would be negotiating with whatever the
    /// caller routed the bytes to.
    pub fn postgres_url(&self) -> String {
        format!(
            "postgres://{POSTGRES_USER}@127.0.0.1:{}/{POSTGRES_DATABASE}?sslmode=disable",
            postgres::LOOPBACK_PORT,
        )
    }
}
