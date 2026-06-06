//! `agents tags add` — bare-naked handler. Writes a PENDING row to
//! `tags.sqlite`, with the parent scope defaulting to the cli's own
//! `Config.agent_instance_hierarchy` (mirrors the optional-parent
//! fallback used by `agents message`).

use objectiveai_sdk::cli::command::agents::tags::add::{Request, Response};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::db;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let parent = request
        .parent_agent_instance_hierarchy
        .clone()
        .unwrap_or_else(|| ctx.config.agent_instance_hierarchy.clone());
    db::tags::upsert_pending_async(
        ctx.filesystem.clone(),
        request.name.clone(),
        request.agent_full_id.clone(),
        parent.clone(),
    )
    .await?;
    Ok(Response {
        name: request.name,
        agent_full_id: request.agent_full_id,
        parent_agent_instance_hierarchy: parent,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::tags::add as sdk;
    use objectiveai_sdk::cli::command::agents::tags::add::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::tags::add as sdk;
    use objectiveai_sdk::cli::command::agents::tags::add::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
