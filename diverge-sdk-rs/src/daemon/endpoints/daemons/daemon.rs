//! One remote daemon, as this daemon records it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{DaemonId, Routes};
use crate::daemon::creator::Creator;
use crate::shared::Identity;
use crate::shared::listing::Keyed;

/// One remote daemon this daemon reaches: a record of one account of
/// ours there. The rules:
///
/// 1. **The ID** is how a relay and a connected tool name it: the hash
///    of `daemon`, `identity` and `routes` — see [`DaemonId::of`] — so
///    none of the three ever changes. Only the description and the tags
///    do.
/// 2. **The remote** is `daemon`, the identity the remote daemon answers
///    a handshake as. Several records may name the same remote: each is
///    another account of ours there.
/// 3. **Ours** is `identity`, the identity this daemon presents to it,
///    one of the host's; its issuer is the remote's, the one issuer the
///    handshake between the two speaks.
/// 4. **Reached** through any connected provider whose daemons list
///    holds `daemon` by a route its [`Routes`] let through: the remote is
///    reachable exactly while it is listable so on one of them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Daemon {
    /// The ID: what it reaches and as whom, hashed. Unique among the
    /// records.
    pub id: DaemonId,
    /// What it is, in the caller's words; empty where none was given.
    pub description: String,
    /// The remote daemon, by the identity it answers as.
    pub daemon: Identity,
    /// The identity this daemon presents to it: our account there.
    pub identity: Identity,
    /// The routes it may be reached by: see [`Routes`]. Its members lie
    /// beside the record's own.
    #[serde(flatten)]
    pub routes: Routes,
    /// The tags on it, sorted.
    pub tags: Vec<String>,
    /// When it was added.
    pub created: DateTime<Utc>,
    /// Who added it.
    pub creator: Creator,
    /// Whether this daemon holds a connection to it now.
    pub connected: bool,
}

/// Keyed by the ID.
impl Keyed for Daemon {
    type Key = DaemonId;

    fn key(&self) -> DaemonId {
        self.id
    }
}
