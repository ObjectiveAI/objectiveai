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

use objectiveai_sdk::cli::command::tasks::list::{Plugin, Request, ResponseItem};

use crate::context::Context;
use crate::db;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Vec<ResponseItem>, Error> {
    let parent =
        super::resolve_scope(ctx, request.agent_instance_hierarchy, request.tag).await?;

    let rows = db::tasks::list_schedules(
        &ctx.db,
        &parent,
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
            id: format!("{}-{}", r.name, r.id),
            agent_instance_hierarchy: r.agent_instance_hierarchy,
            command: r.command,
            description: r.description,
            created_at: r.created_at,
            last_ran_at: r.last_ran_at,
            interval: r.interval_seconds.map(|secs| {
                humantime::format_duration(std::time::Duration::from_secs(secs))
                    .to_string()
            }),
            version: r.version as u64,
            plugin: r.plugin.map(|p| Plugin {
                owner: p.owner,
                repository: p.repository,
                version: p.version,
            }),
        })
        .collect())
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::tasks::list as sdk;
    use objectiveai_sdk::cli::command::tasks::list::request_schema::{
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
    use objectiveai_sdk::cli::command::tasks::list as sdk;
    use objectiveai_sdk::cli::command::tasks::list::response_schema::{
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
