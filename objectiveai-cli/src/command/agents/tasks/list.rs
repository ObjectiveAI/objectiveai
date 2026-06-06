//! `agents tasks list` — read schedules with optional filters.
//!
//! Hierarchy scope is resolved at handler time:
//! - `--agent-instance-hierarchy <h>` → that hierarchy.
//! - `--tag <name>` → BOUND lookup in `tags.sqlite`; PENDING /
//!   ABSENT raise structured errors.
//! - Neither set → cli's own `Config.agent_instance_hierarchy`.
//!
//! Everything else (depth, kind, readiness, offset, count) is
//! threaded through to `db::tasks::list_schedules_async` as-is.

use objectiveai_sdk::cli::command::agents::tasks::list::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::db;

pub async fn execute(ctx: &Context, request: Request) -> Result<Vec<ResponseItem>, Error> {
    let parent = match (request.agent_instance_hierarchy, request.tag) {
        (Some(h), None) => h,
        (None, Some(tag)) => {
            use crate::filesystem::db::tags;
            match tags::lookup_async(ctx.filesystem.clone(), tag.clone()).await? {
                tags::LookupState::Bound { agent_instance_hierarchy } => {
                    agent_instance_hierarchy
                }
                tags::LookupState::Pending {
                    parent_agent_instance_hierarchy,
                    agent_full_id,
                } => {
                    return Err(Error::TagPending {
                        tag,
                        parent_agent_instance_hierarchy,
                        agent_full_id,
                    });
                }
                tags::LookupState::Absent => return Err(Error::TagNotFound(tag)),
            }
        }
        (None, None) => ctx.config.agent_instance_hierarchy.clone(),
        (Some(_), Some(_)) => unreachable!(
            "clap group `scope` enforces mutex between --agent-instance-hierarchy / --tag"
        ),
    };

    let rows = db::tasks::list_schedules_async(
        ctx.filesystem.clone(),
        parent,
        request.depth,
        request.oneshot,
        request.interval,
        request.pending,
        request.exhausted,
        request.offset.unwrap_or(0),
        request.count,
    )
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ResponseItem {
            id: r.id,
            agent_instance_hierarchy: r.agent_instance_hierarchy,
            command: r.command,
            description: r.description,
            created_at: r.created_at,
            last_ran_at: r.last_ran_at,
            interval: r.interval_seconds.map(|secs| {
                humantime::format_duration(std::time::Duration::from_secs(secs))
                    .to_string()
            }),
        })
        .collect())
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::tasks::list as sdk;
    use objectiveai_sdk::cli::command::agents::tasks::list::request_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::tasks::list as sdk;
    use objectiveai_sdk::cli::command::agents::tasks::list::response_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::ResponseItem),
        ))
    }
}
