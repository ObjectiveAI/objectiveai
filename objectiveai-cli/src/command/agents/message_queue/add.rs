//! `agents message-queue add` — bare-naked handler. Persists one
//! `RichContent` into the `prompts` table in `tags.sqlite` against
//! either the resolved `{parent}/{instance}` hierarchy (Direct
//! mode) or the literal tag name (Tag mode — no resolution).

use objectiveai_sdk::cli::command::agents::message_queue::add::{Request, Response, Target};

use crate::context::Context;
use crate::db;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    // Resolve the message up-front via `agents message`'s existing
    // helper — turns all five RequestMessage variants into one
    // RichContent. File / Python sources are read NOW so they
    // don't need to survive until the future dequeue.
    let content = crate::command::agents::instances::message::resolve_message(request.message)?;

    // Normalise the target to (Option<full_hierarchy>, Option<tag>).
    // Exactly one is Some; the table's CHECK enforces this at the
    // DB layer too. Direct mode additionally validates that the
    // resolved hierarchy has at least one `agent_completion_request`
    // row logged. Tag mode is intentionally exempt — tags can be
    // enqueued against agents that don't exist yet.
    let (agent_instance_hierarchy, agent_tag) = match request.target {
        Target::Direct {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .as_deref()
                .unwrap_or(&ctx.config.agent_instance_hierarchy);
            let full_id = format!("{parent}/{agent_instance}");
            if !ctx.filesystem.agent_exists(&ctx.db, &full_id).await? {
                return Err(Error::AgentNoPriorRequest {
                    agent_instance_hierarchy: full_id,
                });
            }
            (Some(full_id), None)
        }
        Target::Tag { agent_tag } => (None, Some(agent_tag)),
    };

    let id = db::prompts::enqueue_with_content(
        &ctx.db,
        agent_instance_hierarchy.clone(),
        agent_tag.clone(),
        request.key,
        content,
    )
    .await?;

    Ok(Response {
        id,
        agent_instance_hierarchy,
        agent_tag,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::message_queue::add as sdk;
    use objectiveai_sdk::cli::command::agents::message_queue::add::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::message_queue::add as sdk;
    use objectiveai_sdk::cli::command::agents::message_queue::add::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
