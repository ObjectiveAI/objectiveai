//! Look up the agent definition + latest continuation that
//! belong to a given `agent_instance_hierarchy`.
//!
//! Two sources, one round-trip:
//!
//! - **agent definition** — read from `objectiveai.agent_refs`, the
//!   definition-source registry keyed by the full AIH (EITHER the
//!   `RemotePath` the WF was fetched from OR the inline spec —
//!   blindly upserted at spawn-by-spec lock acquisition and by the
//!   log writer whenever a chunk carries `agent_inline`). This
//!   replaced the old most-recent-request-blob extraction: the
//!   registry is authoritative and needs no `response_id` parsing.
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
/// `Ok(None)` means the `agent_refs` registry holds no definition
/// source for that hierarchy (no prior session recorded).
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

    let Some(row) = row else { return Ok(None) };

    let remote: Option<String> = row.try_get("remote")?;
    let inline: Option<serde_json::Value> = row.try_get("inline")?;
    let continuation: Option<String> = row.try_get("continuation")?;

    // Both variants round-trip through the request-side untagged
    // union: a remote is its wire string, an inline spec its object.
    // The log writer stores the VALIDATED `InlineAgentWithFallbacks`
    // (which serializes its computed `id`s alongside the flattened
    // base); deserializing into the base type simply ignores those
    // extra fields.
    let agent_value = match (remote, inline) {
        (Some(remote), _) => serde_json::Value::String(remote),
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
