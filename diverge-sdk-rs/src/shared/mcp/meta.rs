//! The one key the daemon sets under `_meta`, and as a frontend
//! request's header: who is on the other side.
//!
//! One key, [`KEY`], one JSON object — the caller, or the tool — set
//! by [`attest`] on a `_meta` the daemon forwards or relays. A
//! request carries the caller: an agent of this daemon's, with its
//! image, when its container sent the call; the identity this daemon
//! authorized, and nothing more, when the call came over a connect
//! from outside. An answer carries the tool, with its image and its
//! template — a tool this daemon runs as the run request made it, a
//! tool another daemon runs as that daemon told on the connect. An
//! agent's chunks carry the agent, as the caller it is. The daemon's
//! stamp replaces whatever arrived, either way: a stamp from outside
//! is not trusted. Who sets it, and where, is stated in
//! [`mcp`](super); what each kind carries, in
//! [`attestation`](super::attestation).

use rmcp::model::{MetaObject, RequestParamsMeta};

use super::attestation::Attested;

/// `diverge.network`: the one key, whose value is the
/// [`Caller`](super::attestation::Caller) on a request and the
/// [`Tool`](super::attestation::Tool) on an answer —
/// `{"kind":…,"image":…,…}`, the kind naming which of the four it
/// is, and the image a member of every kind.
pub const KEY: &str = "diverge.network";

/// Set [`KEY`] on `meta` to `attestation` — the caller or the tool,
/// see [`Attested`]. A value already under it is replaced, so that
/// nothing under it attests what the daemon did not; every other key
/// stays as it was sent.
pub fn attest<A: Attested>(meta: &mut MetaObject, attestation: &A) {
    if let Ok(value) = serde_json::to_value(attestation) {
        meta.insert(KEY.to_string(), value);
    }
}

/// The value of the `diverge.network` header the daemon sets on a
/// frontend request: the caller as JSON, every character outside ASCII
/// written as a `\uXXXX` escape (a surrogate pair beyond the basic
/// plane), so the value is a valid HTTP header value and parses to the
/// same [`Caller`](super::attestation::Caller) as the `_meta` value
/// does.
pub fn header(caller: &super::attestation::Caller) -> String {
    let json = serde_json::to_string(caller).unwrap_or_default();
    let mut out = String::with_capacity(json.len());
    for c in json.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out
}

/// [`attest`] on a request's params, whose `_meta` is made when the
/// request carried none.
pub fn attest_request<P: RequestParamsMeta, A: Attested>(params: &mut P, attestation: &A) {
    attest(params.meta_or_default(), attestation);
}
