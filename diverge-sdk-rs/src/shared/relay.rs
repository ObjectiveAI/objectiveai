//! A request served by another daemon: the member every request of
//! the daemon's wire may carry.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::daemons::DaemonId;

/// Where a request is served: through other daemons, by the IDs of this
/// daemon's [records](crate::daemon::endpoints::daemons) of them, and of
/// each next daemon's records in turn. Every request of the daemon's
/// wire carries this as an optional member, `relay`, an array of IDs;
/// absent, or empty, the request is served by the daemon it was sent to.
/// The rules:
///
/// 1. **The first ID is this daemon's.** A daemon that reads a request
///    with `relay` does not serve it: it takes the first ID off, finds
///    its record of that ID, and judges that the caller may relay to it
///    — the daemons grant's `relay` over that record — and nothing else
///    of the request.
/// 2. **The rest is the next daemon's.** It connects to that remote as
///    the account the record names, and puts the request there with the
///    first ID taken off: the rest as the request's `relay`, or no
///    `relay` where none is left. The remote judges the next ID, and
///    then the request itself, as that account's.
/// 3. **A true conduit.** It carries every frame both ways until the
///    remote finishes — the responses, the channels either side opens,
///    their responses and their finishes, byte for byte, reading none of
///    it. The scope and channel numbers are the daemon's to remap, since
///    the client's numbering and the daemon-to-daemon numbering are two
///    relationships; the client sees its own.
/// 4. **Each hop is what the caller meant.** An ID is the hash of what
///    its record reaches and as whom — see [`DaemonId`] — so no daemon
///    along the route can re-point a hop: it holds the record the ID
///    names, reaching exactly that remote as exactly that account, or
///    none, and refuses.
///
/// # What a refusal looks like
///
/// A relay the daemon will not make — no record of the first ID, the
/// caller holds no grant to relay to it, the remote could not be reached
/// — is answered as the request's own `forbidden` or `error`, the one
/// answer every endpoint has, so a client reading the endpoint's own
/// frames reads it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(transparent)]
pub struct Relay(pub Vec<DaemonId>);
