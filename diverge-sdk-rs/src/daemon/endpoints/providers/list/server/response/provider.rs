//! One provider, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::providers::{Direction, Status};
use crate::provider::endpoints::capacity;
use crate::shared::Identity;
use crate::shared::description::Description;
use crate::shared::listing::Keyed;

/// One provider the daemon keeps a record of: its identity, the
/// direction its connection is made in — its members beside the
/// record's own, under `direction` — what the daemon announces itself
/// with there, whether it accepts other daemons and hears messages
/// through it, whether it is enabled, what it is in the
/// caller's words and its tags, who made the record and when, and
/// where the daemon stands with it now — and, connected, what the
/// provider answered its `info` and `capacity` asks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Provider {
    /// The provider's identity: the `(issuer, subject)` its token
    /// establishes in the handshake.
    pub identity: Identity,
    /// How the connection is made: see [`Direction`].
    #[serde(flatten)]
    pub direction: Direction,
    /// What the daemon announces itself with on it: the text of the
    /// `daemons::announce` the daemon holds open there while connected,
    /// by which its `daemons::list` lists the daemon. Absent, the
    /// daemon does not announce itself there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub announcement: Option<Description>,
    /// Whether the daemon accepts connections from other daemons
    /// through it: holds its `daemons::accept` open while connected.
    pub accept_daemons: bool,
    /// Whether the daemon listens for messages on it: holds its
    /// `daemons::listen` open while connected.
    pub messageable: bool,
    /// Whether the daemon works with it: dials it, or admits it.
    pub enabled: bool,
    /// What it is, in the caller's words — the operator's note, which
    /// the provider never sees; empty where none was given.
    pub description: String,
    /// The tags on it, sorted.
    pub tags: Vec<String>,
    /// When the record was made.
    pub created: DateTime<Utc>,
    /// Who made it.
    pub creator: Creator,
    /// Where the daemon stands with it now: see [`Status`].
    pub status: Status,
    /// The provider's latest answer to the daemon's `capacity` ask, as
    /// the daemon cached it: see
    /// [`Frame`](crate::provider::endpoints::capacity::server::response::Frame).
    /// Absent until the provider has answered once, and whenever the
    /// status is not `connected`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<capacity::server::response::Frame>,
}

/// Keyed by the identity.
impl Keyed for Provider {
    type Key = Identity;

    fn key(&self) -> Identity {
        self.identity.clone()
    }
}
