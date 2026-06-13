//! Look up the agent definition + latest continuation that
//! belong to a given `agent_instance_hierarchy`.
//!
//! Two sources, one round-trip:
//!
//! - **agent definition** — extracted from
//!   `objectiveai.agent_completion_requests.body.agent` (the request blob
//!   is a serialized `AgentCompletionCreateParams`). The request
//!   row is PK'd by `response_id`, which is the trailing suffix of
//!   the AIH after the final `-`
//!   (`{ctx lineage}/{agent_full_id}-{response_id}`).
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
/// `Ok(None)` means there's no logged request for that
/// hierarchy's embedded `response_id` (no prior session).
pub async fn lookup_session(
    pool: &Pool,
    agent_instance_hierarchy: &str,
) -> Result<Option<SessionLookup>, Error> {
    // Split on the FINAL `-`: everything after is the response_id.
    // No `-` at all means the hierarchy doesn't carry a response_id
    // suffix and can't be resolved — return None.
    let Some((_, response_id)) = agent_instance_hierarchy.rsplit_once('-') else {
        return Ok(None);
    };

    // LEFT JOIN `agent_continuations` onto the request row. The
    // request row is PK'd by `response_id`; the continuation row is
    // PK'd by the full AIH. Both keys are bound separately. A NULL
    // `continuation` column means there's no row in
    // `agent_continuations` yet for this AIH.
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
