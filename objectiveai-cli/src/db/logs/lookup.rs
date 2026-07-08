//! Look up the agent definition + latest continuation that
//! belong to a given `agent_instance_hierarchy`.
//!
//! Sources, in order:
//!
//! - **agent definition** — read from `objectiveai.agent_refs`, the
//!   definition-source registry keyed by the full AIH (EITHER the
//!   `RemotePath` the WF was fetched from OR the inline spec —
//!   blindly upserted at spawn-by-spec lock acquisition and by the
//!   log writer whenever a chunk carries `agent_inline`). When the
//!   registry has no row — agents whose last activity predates the
//!   table — the MIGRATION FALLBACK extracts the definition from
//!   the most recent `agent_completion_requests` blob, keyed by the
//!   `response_id` suffix embedded in the AIH. Any post-table run
//!   repopulates the registry, so the fallback decays naturally.
//! - **latest continuation** — read straight from the
//!   `agent_continuations` table keyed by the full AIH. The
//!   chunk-yielder loops (`agents spawn` + `functions execute`)
//!   upsert into that table per chunk, so this is the
//!   authoritative latest-continuation source — no more parsing it
//!   out of the cumulative response blob.
//!
//! Used by `agents message`'s stream-true path after it acquires
//! the hierarchy's lock: it needs the agent definition to drive
//! `spawn::run_multi_pass` and the latest continuation to seed the
//! resumed conversation.

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use sqlx::Row as _;

use super::super::{Error, Pool};

/// What [`lookup_session`] returns when a prior session exists for
/// the queried `agent_instance_hierarchy`.
#[derive(Debug, Clone)]
pub struct SessionLookup {
    pub agent: InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    /// `None` when the `agent_continuations` row hasn't been
    /// written yet — typically because the stream errored before
    /// any chunk that carried a continuation landed.
    pub continuation: Option<String>,
}

/// Resolve the session for `agent_instance_hierarchy`.
/// `Ok(None)` means neither the `agent_refs` registry nor the
/// legacy request-blob fallback holds a definition for that
/// hierarchy (no prior session recorded).
pub async fn lookup_session(
    pool: &Pool,
    agent_instance_hierarchy: &str,
) -> Result<Option<SessionLookup>, Error> {
    // One row per AIH in `agent_refs` (remote XOR inline, CHECK-
    // enforced); LEFT JOIN the continuation keyed by the same AIH. A
    // NULL `continuation` column means there's no row in
    // `agent_continuations` yet for this AIH.
    let row = sqlx::query(
        "SELECT refs.remote AS remote, refs.inline AS inline, \
                cont.continuation AS continuation \
         FROM objectiveai.agent_refs refs \
         LEFT JOIN objectiveai.agent_continuations cont \
           ON cont.agent_instance_hierarchy = refs.agent_instance_hierarchy \
         WHERE refs.agent_instance_hierarchy = $1",
    )
    .bind(agent_instance_hierarchy)
    .fetch_optional(&**pool)
    .await?;

    let Some(row) = row else {
        // Migration fallback: pre-`agent_refs` agents resolve from
        // their most recent request blob.
        return lookup_session_from_request_blob(pool, agent_instance_hierarchy)
            .await;
    };

    let remote: Option<String> = row.try_get("remote")?;
    let inline: Option<serde_json::Value> = row.try_get("inline")?;
    let continuation: Option<String> = row.try_get("continuation")?;

    // Both variants round-trip through the request-side untagged
    // union. A remote's canonical form is `RemotePathCommitOptional`'s
    // TAGGED OBJECT (`{"remote":"client","owner":…}`) — there is no
    // string wire form — so the column holds its JSON text and is
    // parsed back as JSON here (wrapping it in `Value::String` can
    // never match the union and loses the definition). A plain
    // non-JSON string falls back to `Value::String` untouched. The
    // log writer stores the VALIDATED `InlineAgentWithFallbacks`
    // (which serializes its computed `id`s alongside the flattened
    // base); deserializing into the base type simply ignores those
    // extra fields.
    let agent_value = match (remote, inline) {
        (Some(remote), _) => serde_json::from_str(&remote)
            .unwrap_or(serde_json::Value::String(remote)),
        (None, Some(inline)) => inline,
        (None, None) => {
            return Err(Error::InvalidData(format!(
                "agent_refs row for {agent_instance_hierarchy} has neither \
                 remote nor inline",
            )));
        }
    };
    let agent: InlineAgentBaseWithFallbacksOrRemoteCommitOptional =
        serde_json::from_value(agent_value)?;

    Ok(Some(SessionLookup { agent, continuation }))
}

/// The pre-`agent_refs` resolution, kept as a migration fallback:
/// extract the definition from
/// `objectiveai.agent_completion_requests.body.agent` (the request
/// blob is a serialized `AgentCompletionCreateParams`). The request
/// row is PK'd by `response_id` — the trailing suffix of the AIH
/// after the final `-`
/// (`{ctx lineage}/{agent_full_id}-{response_id}`).
async fn lookup_session_from_request_blob(
    pool: &Pool,
    agent_instance_hierarchy: &str,
) -> Result<Option<SessionLookup>, Error> {
    // Split on the FINAL `-`: everything after is the response_id.
    // No `-` at all means the hierarchy doesn't carry a response_id
    // suffix and can't be resolved — return None.
    let Some((_, response_id)) = agent_instance_hierarchy.rsplit_once('-') else {
        return Ok(None);
    };

    let row = sqlx::query(
        "SELECT req.body AS request_body, cont.continuation AS continuation \
         FROM objectiveai.agent_completion_requests req \
         LEFT JOIN objectiveai.agent_continuations cont \
           ON cont.agent_instance_hierarchy = $2 \
         WHERE req.response_id = $1",
    )
    .bind(response_id)
    .bind(agent_instance_hierarchy)
    .fetch_optional(&**pool)
    .await?;

    let Some(row) = row else { return Ok(None) };

    let request_body: serde_json::Value = row.try_get("request_body")?;
    let continuation: Option<String> = row.try_get("continuation")?;

    // The request blob is a serialized
    // `AgentCompletionCreateParams`; the agent field there is what
    // spawn's `Request.agent` carries.
    let agent_value = request_body.get("agent").cloned().ok_or_else(|| {
        Error::InvalidData(format!(
            "agent_completion_requests.body missing `agent` field for response_id {response_id}",
        ))
    })?;
    let agent: InlineAgentBaseWithFallbacksOrRemoteCommitOptional =
        serde_json::from_value(agent_value)?;

    Ok(Some(SessionLookup { agent, continuation }))
}
