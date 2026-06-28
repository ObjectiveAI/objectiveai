//! `agents tags apply` — bare-naked handler. Binds a tag against one
//! of three targets:
//!
//! - `Target::AgentInstance` — writes a BOUND row pointing at
//!   `{parent}/{agent_instance}`. Parent defaults to ctx own.
//! - `Target::Agent` — creates a fresh `tag_groups` row carrying the
//!   resolved `AgentSpec` + parent, then a `tags` row pointing at
//!   that group. Parent defaults to ctx own.
//!   are stored as-is — the spawn path resolves them at execution
//!   time (so the tag carries the symbolic reference, not the frozen
//!   snapshot).
//! - `Target::AgentTag` — clones an existing tag's resolution under
//!   the new name. BOUND source → new BOUND row at the same AIH;
//!   GROUPED source → new tag joining the same group. Self-reference
//!   is rejected as a cycle; longer cycles are impossible because
//!   storage is single-step. Parent is forbidden (the source's
//!   parent is inherited via the group).

use objectiveai_sdk::cli::command::agents::tags::apply::{
    AgentTagResolution, Request, Response, Target,
};

use crate::context::Context;
use crate::db;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let resolved = match request.target {
        Target::AgentInstance {
            agent_instance,
            parent_agent_instance_hierarchy,
        } => {
            let parent = parent_agent_instance_hierarchy
                .unwrap_or_else(|| ctx.config.agent_instance_hierarchy.clone());
            db::tags::ResolvedApplyTarget::AgentInstance {
                parent_agent_instance_hierarchy: parent,
                agent_instance,
            }
        }
        Target::Agent {
            agent_spec,
            parent_agent_instance_hierarchy,
        } => {
            let parent = parent_agent_instance_hierarchy
                .unwrap_or_else(|| ctx.config.agent_instance_hierarchy.clone());
            db::tags::ResolvedApplyTarget::Agent {
                parent_agent_instance_hierarchy: parent,
                agent_spec,
            }
        }
        Target::AgentTag { agent_tag } => {
            db::tags::ResolvedApplyTarget::AgentTag { agent_tag }
        }
    };
    // Capture the source-tag name (if any) ahead of the move into
    // `apply`, so we can surface it in the `AgentTag` response arm.
    let source_tag = match &resolved {
        db::tags::ResolvedApplyTarget::AgentTag { agent_tag } => Some(agent_tag.clone()),
        _ => None,
    };
    // Resolve the db handle before taking the lock so an error there can't
    // skip the explicit release below.
    let pool = ctx.db_client().await?;
    // Laboratories travel with the tag when it's relocated, so a tag may not be
    // moved while an agent holding it is live. Take the tag lock NON-BLOCKING
    // (`try_acquire`): a held lock means a live process owns that tag, and the
    // apply is rejected. Released right after the write — this command isn't an
    // active agent, it just needs exclusivity for the relocation itself.
    let state_dir = ctx.filesystem.state_dir();
    let (lock_dir, lock_key) =
        crate::command::agents::locks::agent_tag_lock(&state_dir, &request.name);
    let Some(claim) = objectiveai_sdk::lockfile::try_acquire(&lock_dir, &lock_key, "").await else {
        return Err(Error::TagApplyAgentActive { tag: request.name });
    };
    let result = db::tags::apply(pool, &request.name, resolved).await;
    // Release on every path (dropping a LockClaim does NOT release it) before
    // propagating the apply outcome.
    claim
        .release()
        .map_err(|e| Error::Lockfile { key: lock_key, source: e })?;
    let state = result?;
    Ok(match (source_tag, state) {
        (None, db::tags::LookupState::Bound { agent_instance_hierarchy }) => {
            // AgentInstance path — re-derive `agent_instance` + parent
            // from the stored AIH so the response stays in lockstep
            // with what the db actually wrote.
            let parent = db::tags::parent_of(&agent_instance_hierarchy).to_string();
            let leaf = db::tags::leaf_of(&agent_instance_hierarchy).to_string();
            Response::AgentInstance {
                name: request.name,
                agent_instance: leaf,
                parent_agent_instance_hierarchy: parent,
                agent_instance_hierarchy,
            }
        }
        (None, db::tags::LookupState::Grouped {
            tag_group_id,
            agent_spec,
            parent_agent_instance_hierarchy,
        }) => Response::Agent {
            name: request.name,
            tag_group_id,
            agent_spec,
            parent_agent_instance_hierarchy,
        },
        (Some(agent_tag), db::tags::LookupState::Bound { agent_instance_hierarchy }) => {
            Response::AgentTag {
                name: request.name,
                agent_tag,
                resolved: AgentTagResolution::Bound { agent_instance_hierarchy },
            }
        }
        (Some(agent_tag), db::tags::LookupState::Grouped {
            tag_group_id,
            agent_spec,
            parent_agent_instance_hierarchy,
        }) => Response::AgentTag {
            name: request.name,
            agent_tag,
            resolved: AgentTagResolution::Grouped {
                tag_group_id,
                agent_spec,
                parent_agent_instance_hierarchy,
            },
        },
        (_, db::tags::LookupState::Absent) => unreachable!(
            "apply() never returns Absent — it just wrote the row"
        ),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::tags::apply as sdk;
    use objectiveai_sdk::cli::command::agents::tags::apply::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::tags::apply as sdk;
    use objectiveai_sdk::cli::command::agents::tags::apply::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
