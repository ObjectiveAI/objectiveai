//! The daemon the viewer dials.

use diverge_sdk::wire::proxy::Redial;
use serde::{Deserialize, Serialize};

/// The daemon the viewer dials, and as whom: the address, and the
/// identity the connection presents — one of the root's
/// [`identities`](diverge_sdk::identities), signed in by
/// `diverge identities login`, whose `(issuer, subject)` is the
/// daemon's account. The document holds no token: the app reads the
/// identity's at start and keeps it fresh, it never reaches the page,
/// which speaks to the daemon only through the app's own Rust, and the
/// daemon's own answer is not judged.
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
    /// Whether the app dials the daemon again by itself when the
    /// connection ends or a dial fails, and how long it waits first:
    /// `never`, or a whole number of seconds, at least `1`. Absent,
    /// every 5 seconds, as the daemon redials a provider — see
    /// [`Redial`].
    #[serde(default, skip_serializing_if = "Redial::is_default")]
    pub redial: Redial,
}
