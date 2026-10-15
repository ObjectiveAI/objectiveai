//! A request served by another daemon: the member every request of
//! the daemon's wire may carry.

use serde::{Deserialize, Serialize};

/// Where a request is served: on another daemon, as one of the
/// accounts this daemon holds on it — a record of
/// [`providers::daemons`](crate::daemon::endpoints::providers::daemons)
/// and one of its accounts, both by name. Every request of the
/// daemon's wire carries this as an optional member, `relay`; absent,
/// the request is served by the daemon it was sent to.
///
/// # A true conduit
///
/// A daemon that reads a request with `relay` does not serve it. It
/// checks that the caller may act as that account on that daemon —
/// the daemons grant's `relay` action over the record, the account
/// among the grant's `accounts` — takes or opens its connection to
/// that daemon as that account, puts the request there with this
/// member taken out, and carries every frame both ways until the
/// remote finishes: the responses, the channels either side opens,
/// their responses and their finishes, byte for byte, reading none
/// of it. The scope and channel numbers are the daemon's to remap,
/// since the client's numbering and the daemon-to-daemon numbering
/// are two relationships; the client sees its own. The remote sees
/// an ordinary request from that account, judged by that account's
/// standing there, and nothing of who sent it here; and since the
/// member is taken out, a request is never relayed twice.
///
/// # What a refusal looks like
///
/// A relay the daemon will not make — the caller holds no grant to
/// act as that account there, no record or account is the one named,
/// the connection could not be opened — is answered as the request's
/// own `forbidden` or `error`, the one answer every endpoint has, so
/// a client reading the endpoint's own frames reads it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Relay {
    /// The daemon, by the name of its record.
    pub daemon: String,
    /// The account on it to act as, by the name the record's accounts
    /// give it.
    pub account: String,
}
