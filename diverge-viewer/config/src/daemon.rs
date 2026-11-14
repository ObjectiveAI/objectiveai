//! The daemon the viewer dials.

use diverge_sdk::shared::Identity;
use diverge_sdk::wire::proxy::Redial;
use serde::{Deserialize, Serialize};

/// The daemon the viewer dials, and as whom: the address, and the
/// identity the connection presents — one of the root's
/// [`identities`](diverge_sdk::identities), signed in by
/// `diverge identities login`, whose `(issuer, subject)` is the
/// daemon's account. The document holds no token: the app reads the
/// identity's at start and keeps it fresh, it never reaches the page,
/// which speaks to the daemon only through the app's own Rust. The
/// daemon's own token is verified, and the daemon held to the key it
/// names; where [`identity`](Self::identity) is stated, it must be
/// that daemon and no other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Daemon {
    /// Where the daemon is dialled: a WebSocket URL, or a bare
    /// `host:port` the viewer puts `ws://` before and `/` after.
    pub address: String,
    /// The identity's issuer: the one the viewer presents its token
    /// from.
    pub issuer: String,
    /// The identity's subject at that issuer.
    pub subject: String,
    /// The daemon's own identity, `{issuer, subject}`: who must answer
    /// at the address. Stated, the handshake fails unless that
    /// identity answers and proves the key its token names. Absent,
    /// whichever identity answers with a token that verifies is taken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<Identity>,
    /// Whether the app dials the daemon again by itself when the
    /// connection ends or a dial fails, and how long it waits first:
    /// `never`, or a whole number of seconds, at least `1`. Absent,
    /// every 5 seconds, as the daemon redials a provider — see
    /// [`Redial`].
    #[serde(default, skip_serializing_if = "Redial::is_default")]
    pub redial: Redial,
}
