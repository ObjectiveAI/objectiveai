//! The one key the daemon sets under `_meta`: who is on the other
//! side.
//!
//! One key, [`KEY`], one JSON object, an [`Attestation`]: set by
//! [`attest`] on a `_meta` the daemon forwards or relays. An agent's
//! calls outward carry the agent, with its image; a tool's answers
//! carry the tool, with its image and its template — a tool this
//! daemon runs as the run request made it, a tool another daemon
//! runs as that daemon told on the connect; an agent's chunks carry
//! the agent. The daemon's stamp replaces whatever an answer arrived
//! with: a stamp from outside is not trusted. Who sets it, and where,
//! is stated in [`mcp`](super); what each kind carries, in
//! [`attestation`](super::attestation).

use rmcp::model::{MetaObject, RequestParamsMeta};

use super::attestation::Attested;

/// `diverge.network`: the one key, whose value is an
/// [`Attestation`](super::attestation::Attestation) —
/// `{"kind":…,"image":…,…}`, the kind naming which of the four it
/// is, and the image a member of every kind.
pub const KEY: &str = "diverge.network";

/// Set [`KEY`] on `meta` to `attestation` — an
/// [`Attestation`](super::attestation::Attestation), or
/// either half of one, see [`Attested`]. A value already under it is
/// replaced, so that nothing under it attests what the daemon did
/// not; every other key stays as it was sent.
pub fn attest<A: Attested>(meta: &mut MetaObject, attestation: &A) {
    if let Ok(value) = serde_json::to_value(attestation) {
        meta.insert(KEY.to_string(), value);
    }
}

/// [`attest`] on a request's params, whose `_meta` is made when the
/// request carried none.
pub fn attest_request<P: RequestParamsMeta, A: Attested>(params: &mut P, attestation: &A) {
    attest(params.meta_or_default(), attestation);
}
