//! What a server's channel request frame carries for a tool
//! container begin.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::{daemon, postgres, vault};

/// What the proxy asks the server for once a tool container
/// has begun.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Postgres`](Self::Postgres) |
/// | `1` | [`Daemon`](Self::Daemon) |
/// | `2` | [`VaultGet`](Self::VaultGet) |
/// | `3` | [`VaultSet`](Self::VaultSet) |
/// | `4` | [`VaultDelete`](Self::VaultDelete) |
/// | `5` | [`VaultLock`](Self::VaultLock) |
/// | `6` | [`VaultUnlock`](Self::VaultUnlock) |
///
/// Seven: everything a tool container asks of the world outside —
/// its database connections, its daemon connection, its vault — which
/// the server relays to the caller as the provider protocol's own
/// channel requests. No tool call outward: a tool container makes
/// none, and a proxy serving one answers its program's MCP client
/// with an error; an agent container's begin has those asks and no
/// daemon connection. The fuse asks are not here; each rides the
/// scope of the mount it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// The proxy's half of a database connection the container's
    /// driver opened. Tag `0`.
    ///
    /// An id the proxy minted. What comes back is everything the
    /// database says; the server opens the other half, or declines by
    /// finishing this one with nothing before it. See
    /// [`postgres`](crate::shared::containers::postgres).
    Postgres(postgres::request::Postgres),
    /// A daemon connection the program opened, by the id the proxy
    /// minted. Tag `1`.
    ///
    /// This half is the proxy's own. What comes back on it is
    /// everything the DAEMON says, as
    /// [`daemon::server::Frame`](crate::shared::containers::daemon::server::Frame)s;
    /// its finish is the connection closed, and an empty finish is the
    /// connection declined. See
    /// [`daemon`](crate::shared::containers::daemon) for the pair.
    Daemon(daemon::request::Daemon),
    /// Read a vault key. Tag `2`.
    VaultGet(vault::get::request::Request<'a>),
    /// Write a vault key. Tag `3`.
    VaultSet(vault::set::request::Request<'a>),
    /// Remove a vault key. Tag `4`.
    VaultDelete(vault::delete::request::Request<'a>),
    /// Hold a vault key's lock. Tag `5`.
    VaultLock(vault::lock::request::Request<'a>),
    /// Release a vault key's lock. Tag `6`.
    ///
    /// The five vault asks are the container's; see
    /// [`vault`](crate::shared::containers::vault) for what each
    /// carries and how a lock behaves.
    VaultUnlock(vault::unlock::request::Request<'a>),
}

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 0;

/// Tag for [`Frame::Daemon`].
const DAEMON: u8 = 1;

/// Tag for [`Frame::VaultGet`].
const VAULT_GET: u8 = 2;

/// Tag for [`Frame::VaultSet`].
const VAULT_SET: u8 = 3;

/// Tag for [`Frame::VaultDelete`].
const VAULT_DELETE: u8 = 4;

/// Tag for [`Frame::VaultLock`].
const VAULT_LOCK: u8 = 5;

/// Tag for [`Frame::VaultUnlock`].
const VAULT_UNLOCK: u8 = 6;

impl Encode for Frame<'_> {
    /// A vault key too long for its prefix; everything else is bytes
    /// copied.
    type Error = FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Postgres(request) => {
                out.extend_from_slice(&[POSTGRES]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                request.encode(out).map_err(|error| match error {})
            }
            Frame::Daemon(request) => {
                out.extend_from_slice(&[DAEMON]);
                request.encode(out).map_err(|error| match error {})
            }
            Frame::VaultGet(request) => {
                out.extend_from_slice(&[VAULT_GET]);
                request.encode(out).map_err(|error| match error {})
            }
            Frame::VaultSet(request) => {
                out.extend_from_slice(&[VAULT_SET]);
                request.encode(out).map_err(FrameEncodeError::Vault)
            }
            Frame::VaultDelete(request) => {
                out.extend_from_slice(&[VAULT_DELETE]);
                request.encode(out).map_err(|error| match error {})
            }
            Frame::VaultLock(request) => {
                out.extend_from_slice(&[VAULT_LOCK]);
                request.encode(out).map_err(|error| match error {})
            }
            Frame::VaultUnlock(request) => {
                out.extend_from_slice(&[VAULT_UNLOCK]);
                request.encode(out).map_err(|error| match error {})
            }
        }
    }
}

/// A tools begin channel request that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// A vault ask would not encode: a key too long for its prefix.
    Vault(vault::RequestEncodeError),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Vault(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Vault(error) => Some(error),
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Several ways to fail, and each names which ask it was.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            POSTGRES => postgres::request::Postgres::decode(rest)
                .map(Frame::Postgres)
                .map_err(FrameError::Postgres),
            DAEMON => daemon::request::Daemon::decode(rest)
                .map(Frame::Daemon)
                .map_err(FrameError::Daemon),
            VAULT_GET => vault::get::request::Request::decode(rest)
                .map(Frame::VaultGet)
                .map_err(FrameError::Vault),
            VAULT_SET => vault::set::request::Request::decode(rest)
                .map(Frame::VaultSet)
                .map_err(FrameError::Vault),
            VAULT_DELETE => vault::delete::request::Request::decode(rest)
                .map(Frame::VaultDelete)
                .map_err(FrameError::Vault),
            VAULT_LOCK => vault::lock::request::Request::decode(rest)
                .map(Frame::VaultLock)
                .map_err(FrameError::Vault),
            VAULT_UNLOCK => vault::unlock::request::Request::decode(rest)
                .map(Frame::VaultUnlock)
                .map_err(FrameError::Vault),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A tools begin channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's seven.
    UnknownTag(u8),
    /// The connection id was not four bytes.
    Postgres(postgres::request::PostgresError),
    /// A vault ask did not decode.
    Vault(vault::RequestError),
    /// The daemon connection's id was not four bytes.
    Daemon(daemon::request::DaemonError),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("tools begin channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown tools begin channel request tag {tag}")
            }
            FrameError::Postgres(error) => write!(f, "{error}"),
            FrameError::Vault(error) => write!(f, "{error}"),
            FrameError::Daemon(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Daemon(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Vault(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
