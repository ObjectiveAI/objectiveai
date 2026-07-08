//! Definition-source registry keyed by `agent_instance_hierarchy`:
//! EITHER the `RemotePath` the agent's WF was fetched from, OR the
//! inline WF spec itself — exactly one per row.
//!
//! Single function: [`upsert`] — blind, last-write-wins, no
//! read-before-write (mirrors [`super::agent_continuations`]). Two
//! writers:
//!
//! - `agents spawn`, for spawns by SPEC only (never via tag or AIH),
//!   at the moment the AIH lock is acquired (first-chunk identity
//!   capture).
//! - The log writer, whenever a chunk carries `agent_inline` (the
//!   first chunk of every agent completion, at any tier/nesting):
//!   `agent_remote` present → [`AgentRefValue::Remote`], else
//!   [`AgentRefValue::Inline`].
//!
//! No read API today — write-only scaffolding for surfacing agent
//! definition sources (e.g. `agents instances get`) later.

use serde::Serialize;

use super::{Error, Pool};

/// One row's payload: the remote path's wire string, or the inline
/// WF spec as JSON.
pub enum AgentRefValue {
    Remote(String),
    Inline(serde_json::Value),
}

impl AgentRefValue {
    /// The remote path's JSON text. `RemotePath` and its
    /// commit-optional twin serialize as TAGGED OBJECTS
    /// (`{"remote":"client","owner":…}`) — there is no string wire
    /// form — so the column stores the object's JSON text and the
    /// reader ([`super::logs::lookup_session`]) parses it back as
    /// JSON. A string-serializing value is stored bare (the reader's
    /// plain-string fallback picks it up).
    pub fn remote<T: Serialize>(remote: &T) -> Option<Self> {
        match serde_json::to_value(remote).ok()? {
            serde_json::Value::String(s) => Some(Self::Remote(s)),
            other => Some(Self::Remote(other.to_string())),
        }
    }

    /// The inline WF spec's JSON form.
    pub fn inline<T: Serialize>(spec: &T) -> Option<Self> {
        serde_json::to_value(spec).ok().map(Self::Inline)
    }
}

fn now_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Insert-or-replace the definition source for an AIH. Blind: prior
/// values are overwritten unconditionally, whichever variant they
/// held.
pub async fn upsert(
    pool: &Pool,
    agent_instance_hierarchy: &str,
    value: AgentRefValue,
) -> Result<(), Error> {
    let (remote, inline) = match value {
        AgentRefValue::Remote(remote) => (Some(remote), None),
        AgentRefValue::Inline(inline) => (None, Some(inline)),
    };
    sqlx::query(
        "INSERT INTO objectiveai.agent_refs \
             (agent_instance_hierarchy, remote, inline, updated_at) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (agent_instance_hierarchy) DO UPDATE SET \
             remote     = EXCLUDED.remote, \
             inline     = EXCLUDED.inline, \
             updated_at = EXCLUDED.updated_at",
    )
    .bind(agent_instance_hierarchy)
    .bind(remote)
    .bind(inline)
    .bind(now_seconds())
    .execute(&**pool)
    .await?;
    Ok(())
}
