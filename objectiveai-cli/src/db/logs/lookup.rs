//! Look up the agent definition + latest continuation that
//! belong to a given `agent_instance_hierarchy`.
//!
//! `agent_instance_hierarchy` embeds the original `response_id`
//! as the trailing suffix after the final `-`
//! (`{ctx lineage}/{agent_full_id}-{response_id}`). Splitting on
//! that delimiter gives us the key both `agent_completion_requests`
//! and `agent_completion_responses` are PK'd by, so one round-
//! trip via a LEFT JOIN returns:
//!
//! - **agent definition** — extracted from
//!   `agent_completion_requests.body.agent` (the request blob is
//!   a serialized `AgentCompletionCreateParams`).
//! - **latest continuation** — extracted from
//!   `agent_completion_responses.body.continuation` (the
//!   coalesced response chunk body, present only after the
//!   stream has emitted at least one chunk that carried a
//!   continuation token; `None` otherwise).
//!
//! Used by `agents instances message`'s stream-true path after
//! it acquires the hierarchy's lock: it needs the agent
//! definition to drive `spawn::run_multi_pass` and the latest
//! continuation to seed the resumed conversation.

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use sqlx::Row as _;

use super::super::{Error, Pool};

/// What [`lookup_session`] returns when a prior session exists for
/// the queried `agent_instance_hierarchy`.
#[derive(Debug, Clone)]
pub struct SessionLookup {
    pub agent: InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    /// `None` when the session has a logged request but no logged
    /// response yet (e.g. the stream errored before its first
    /// chunk landed).
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

    // LEFT JOIN so a missing response row still gives us the
    // request row's `body`. `response_id` is PK on both tables —
    // at most one row each side.
    let row = sqlx::query(
        "SELECT req.body AS request_body, \
                resp.body AS response_body \
         FROM logs.agent_completion_requests req \
         LEFT JOIN logs.agent_completion_responses resp \
           ON resp.response_id = req.response_id \
         WHERE req.response_id = $1",
    )
    .bind(response_id)
    .fetch_optional(&**pool)
    .await?;

    let Some(row) = row else { return Ok(None) };

    let request_body: serde_json::Value = row.try_get("request_body")?;
    let response_body: Option<serde_json::Value> = row.try_get("response_body")?;

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

    // Continuation is the trailing chunk's `continuation` field
    // (the response body is the COALESCED chunk view, so the
    // continuation persisted there is the latest one emitted).
    let continuation = response_body
        .as_ref()
        .and_then(|b| b.get("continuation"))
        .and_then(|c| c.as_str())
        .map(str::to_string);

    Ok(Some(SessionLookup { agent, continuation }))
}
