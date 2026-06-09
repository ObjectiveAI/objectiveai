//! `agents tasks schedule` — bare-naked handler. Captures the
//! caller's current `AgentArguments` from env vars and persists
//! the schedule row in `tasks.sqlite`.

use objectiveai_sdk::cli::command::AgentArguments;
use objectiveai_sdk::cli::command::tasks::schedule::{Request, Response};

use crate::context::Context;
use crate::db;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    // Snapshot the caller's identity from `ctx.config` — the CLI
    // bin reads OBJECTIVEAI_* env vars into Config at startup, so
    // this captures the same identity without re-touching the env.
    // The runner that fires this schedule later will re-export each
    // `Some(_)` field as the matching env var.
    let agent_arguments = AgentArguments {
        agent_instance_hierarchy: Some(ctx.config.agent_instance_hierarchy.clone()),
        agent_id: ctx.config.agent_id.clone(),
        agent_full_id: ctx.config.agent_full_id.clone(),
        agent_remote: ctx.config.agent_remote.clone(),
        response_id: ctx.config.response_id.clone(),
        response_ids: ctx.config.response_ids.clone(),
        mcp_session_id: ctx.config.mcp_session_id.clone(),
    };

    let name = request.name.clone();
    let db_id = db::tasks::insert_schedule(
        &ctx.db,
        &request.name,
        &request.command,
        &request.description,
        &ctx.config.agent_instance_hierarchy,
        request.interval_seconds,
        &agent_arguments,
    )
    .await?;

    Ok(Response { id: format!("{name}-{db_id}") })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::tasks::schedule as sdk;
    use objectiveai_sdk::cli::command::tasks::schedule::request_schema::{
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
    use objectiveai_sdk::cli::command::tasks::schedule as sdk;
    use objectiveai_sdk::cli::command::tasks::schedule::response_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
