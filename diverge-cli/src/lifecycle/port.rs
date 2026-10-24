//! The port a stop is asked on.

use std::path::Path;

use super::{Error, Program};
use crate::config;

/// The port `program` listens on, on this host, which is where a
/// `stop` is put.
///
/// - The daemon's is the port of the daemon `cli.yaml` names as its
///   default — the document read as every command reads it, see
///   [`config`] — since the daemon on the root is the one the CLI
///   dials by default. The address is `ws://<host>:<port>/`, or
///   `<host>:<port>` with no scheme, the host `127.0.0.1`,
///   `localhost` or `::1`: a stop is honoured only from this host, so
///   an address that names another is [`Error::NotLocal`] rather than
///   a request that would be refused. A port that is absent is the
///   daemon's own default,
///   [`DEFAULT_PORT`](diverge_sdk::daemon::DEFAULT_PORT), and one that
///   is not a number is [`Error::Address`].
/// - The provider's is `port` in `<root>/provider.yaml`, read as the
///   provider reads it — the default when there is no file — through
///   its own crate, so the port is the one the provider is listening
///   on; a file the provider would refuse is [`Error::Provider`].
/// - The issuer's is `port` in `<root>/issuer.yaml`, read as the
///   issuer reads it, through its own crate; a file the issuer would
///   refuse is [`Error::Issuer`].
///
/// `daemon.yaml` is never consulted: the daemon's own port is in
/// `cli.yaml`, where the CLI dials it, and the Postgres a daemon
/// started is the daemon's to stop.
pub async fn port(root: &Path, program: Program) -> Result<u16, Error> {
    match program {
        Program::Daemon => {
            let config = config::load(root).await?;
            let selected = config::select(&config, None, None)?;
            daemon(&selected.daemon.address)
        }
        Program::Provider => {
            let config = diverge_provider::config::load(root)
                .await
                .map_err(Error::Provider)?;
            Ok(config.port)
        }
        Program::Issuer => {
            let config = diverge_issuer::config::load(root)
                .await
                .map_err(Error::Issuer)?;
            Ok(config.port)
        }
    }
}

/// The port in a daemon's `address`, as the rules above state.
fn daemon(address: &str) -> Result<u16, Error> {
    let rest = match address.split_once("://") {
        Some((_, rest)) => rest,
        None => address,
    };
    let authority = rest.split('/').next().unwrap_or_default();
    let (host, port) = split(authority);
    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        return Err(Error::NotLocal(address.to_string()));
    }
    match port {
        None => Ok(diverge_sdk::daemon::DEFAULT_PORT),
        Some(port) => port
            .parse()
            .map_err(|_| Error::Address(address.to_string())),
    }
}

/// `authority` as its host and its port, the port absent when there
/// is none: `host:port`, `host`, `[v6]:port` or `[v6]`, the brackets
/// of a bare IPv6 host taken off.
fn split(authority: &str) -> (&str, Option<&str>) {
    if let Some(rest) = authority.strip_prefix('[') {
        return match rest.split_once(']') {
            Some((host, port)) => (host, port.strip_prefix(':')),
            None => (rest, None),
        };
    }
    match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    }
}
