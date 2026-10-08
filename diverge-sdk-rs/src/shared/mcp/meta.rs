//! The keys the daemon sets under `_meta`: who is on the other side.
//!
//! Three keys, one JSON object each, set by [`attest`] on a `_meta`
//! the daemon forwards or relays: the image, the agent, the tool. An
//! agent's calls outward carry the agent's image and the agent's key;
//! a tool's answers carry the tool's image, when the daemon knows it,
//! and the tool's key; an agent's chunks carry the agent's. Who sets
//! them, and where, is stated in [`mcp`](super).

use rmcp::model::{MetaObject, RequestParamsMeta};

use crate::daemon::key;
use crate::shared::containers::request::Image;

/// `diverge.network/image`: the image the container on the other
/// side was made from, `{"name":…,"digest":…}` as the run request
/// named it. Absent when the daemon does not know it — a tool it
/// joined through its provider rather than ran.
pub const IMAGE: &str = "diverge.network/image";

/// `diverge.network/agent`: the agent on the other side, as a
/// [`key::Agent`] — `{"template":…,"index":…}`, and its `name` when
/// it has one. Set on what an agent sends and says; never beside
/// [`TOOL`].
pub const AGENT: &str = "diverge.network/agent";

/// `diverge.network/tool`: the tool on the other side, as a
/// [`key::Tool`] — `{"origin":…,"index":…}`, and its `name` when it
/// has one; the origin tagged `kind`, `created` with its `template`
/// or `connected` with its `provider` and `id`. Set on what a tool
/// answers; never beside [`AGENT`].
pub const TOOL: &str = "diverge.network/tool";

/// Which container is attested: the one agent, or the one tool.
#[derive(Debug, Clone, Copy)]
pub enum Who<'a> {
    /// An agent: what it sends outward, and what it says.
    Agent(&'a key::Agent),
    /// A tool: what its server answers.
    Tool(&'a key::Tool),
}

/// Set the keys on `meta`: [`IMAGE`] to `image`, and [`AGENT`] or
/// [`TOOL`] to `who`. A value already under any of the three is
/// replaced, and the two that do not apply — [`IMAGE`] with no
/// image, the key of the other kind — are removed, so that nothing
/// under them attests what the daemon did not; every other key stays
/// as it was sent.
pub fn attest(meta: &mut MetaObject, image: Option<&Image>, who: Who<'_>) {
    match image.and_then(|image| serde_json::to_value(image).ok()) {
        Some(image) => {
            meta.insert(IMAGE.to_string(), image);
        }
        None => {
            meta.remove(IMAGE);
        }
    }
    let (key, other, value) = match who {
        Who::Agent(agent) => (AGENT, TOOL, serde_json::to_value(agent)),
        Who::Tool(tool) => (TOOL, AGENT, serde_json::to_value(tool)),
    };
    meta.remove(other);
    if let Ok(value) = value {
        meta.insert(key.to_string(), value);
    }
}

/// [`attest`] on a request's params, whose `_meta` is made when the
/// request carried none.
pub fn attest_request<P: RequestParamsMeta>(params: &mut P, image: Option<&Image>, who: Who<'_>) {
    attest(params.meta_or_default(), image, who);
}
