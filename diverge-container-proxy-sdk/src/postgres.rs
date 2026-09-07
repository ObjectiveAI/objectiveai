//! Postgres: where the driver dials.
//!
//! The proxy is a database to the program beside it — a pgwire
//! listener on the container's loopback that carries every connection
//! to the caller's own database, raw, one connection one session. No
//! method here speaks the protocol: the program's driver does, as it
//! would to any Postgres, and the one thing it needs from this crate
//! is the address.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use diverge_provider_sdk::container_proxy::postgres;

use crate::Client;

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
}
