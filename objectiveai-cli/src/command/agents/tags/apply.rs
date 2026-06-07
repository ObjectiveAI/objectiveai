//! `agents tags apply` — bare-naked handler. Binds (or queues) a tag
//! against one of three targets:
//!
//! - `Target::Me` — writes a BOUND row pointing at the cli's own
//!   `Config.agent_instance_hierarchy`.
//! - `Target::AgentFullId` — writes a PENDING row keyed by
//!   `(agent_full_id, parent)`. Parent defaults to ctx own.
//!   `streaming.rs` auto-promotes to BOUND on the next matching
//!   first chunk.
//! - `Target::AgentInstance` — writes a BOUND row pointing at
//!   `{parent}/{instance}` (or just `{instance}` when parent is
//!   empty, matching the rootless case in `tags::upgrade`). Parent
//!   defaults to ctx own.
//! - `Target::AgentTag` — looks up an existing tag and writes a new
//!   row under `request.name` that reproduces the source row's state
//!   (BOUND or PENDING). PENDING aliases auto-promote alongside the
//!   source on the next matching spawn — `tags::upgrade` UPDATEs by
//!   `(agent_full_id, parent)` with no `name` filter. Absent source
//!   raises `Error::TagNotFound`.

use objectiveai_sdk::cli::command::agents::tags::apply::{
    AgentTagResolution, Request, Response, Target,
};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::db;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    match request.target {
        Target::Me => {
            let hierarchy = ctx.config.agent_instance_hierarchy.clone();
            db::tags::upsert_bound_async(
                ctx.filesystem.clone(),
                request.name.clone(),
                hierarchy.clone(),
            )
            .await?;
            Ok(Response::Me {
                name: request.name,
                agent_instance_hierarchy: hierarchy,
            })
        }
        Target::AgentFullId {
            agent_full_id,
            parent_agent_instance_hierarchy,
        } => {
            let parent = parent_agent_instance_hierarchy
                .unwrap_or_else(|| ctx.config.agent_instance_hierarchy.clone());
            db::tags::upsert_pending_async(
                ctx.filesystem.clone(),
                request.name.clone(),
                agent_full_id.clone(),
                parent.clone(),
            )
            .await?;
            Ok(Response::AgentFullId {
                name: request.name,
                agent_full_id,
                parent_agent_instance_hierarchy: parent,
            })
        }
        Target::AgentInstance {
            agent_instance,
            parent_agent_instance_hierarchy,
        } => {
            let parent = parent_agent_instance_hierarchy
                .unwrap_or_else(|| ctx.config.agent_instance_hierarchy.clone());
            // Rootless guard: empty parent yields just the instance,
            // matching how `tags::upgrade` writes promoted rootless
            // rows (see objectiveai-cli/src/filesystem/db/tags.rs).
            let hierarchy = if parent.is_empty() {
                agent_instance.clone()
            } else {
                format!("{parent}/{agent_instance}")
            };
            db::tags::upsert_bound_async(
                ctx.filesystem.clone(),
                request.name.clone(),
                hierarchy.clone(),
            )
            .await?;
            Ok(Response::AgentInstance {
                name: request.name,
                agent_instance,
                parent_agent_instance_hierarchy: parent,
                agent_instance_hierarchy: hierarchy,
            })
        }
        Target::AgentTag { agent_tag } => {
            // Snapshot the source tag's state, then write a new row
            // under `request.name` reproducing it. Two transactions
            // — a concurrent first-chunk promotion between them may
            // leave the alias one state behind (PENDING after source
            // flipped to BOUND). Acceptable: re-applying re-snapshots.
            // PENDING aliases auto-promote on the next matching spawn
            // because `tags::upgrade` matches by (agent_full_id,
            // parent) and writes every matching row.
            let state = db::tags::lookup_async(
                ctx.filesystem.clone(),
                agent_tag.clone(),
            )
            .await?;
            let resolved = match state {
                db::tags::LookupState::Bound { agent_instance_hierarchy } => {
                    db::tags::upsert_bound_async(
                        ctx.filesystem.clone(),
                        request.name.clone(),
                        agent_instance_hierarchy.clone(),
                    )
                    .await?;
                    AgentTagResolution::Bound { agent_instance_hierarchy }
                }
                db::tags::LookupState::Pending {
                    parent_agent_instance_hierarchy,
                    agent_full_id,
                } => {
                    db::tags::upsert_pending_async(
                        ctx.filesystem.clone(),
                        request.name.clone(),
                        agent_full_id.clone(),
                        parent_agent_instance_hierarchy.clone(),
                    )
                    .await?;
                    AgentTagResolution::Pending {
                        agent_full_id,
                        parent_agent_instance_hierarchy,
                    }
                }
                db::tags::LookupState::Absent => {
                    return Err(Error::TagNotFound(agent_tag));
                }
            };
            Ok(Response::AgentTag {
                name: request.name,
                agent_tag,
                resolved,
            })
        }
    }
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
