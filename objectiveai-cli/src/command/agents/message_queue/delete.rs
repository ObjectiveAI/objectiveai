//! `agents message-queue delete` — bare-naked handler. Atomically
//! drops one queued prompt by its `prompts.id` and returns the
//! deleted row's metadata + reconstructed content body. Cascade on
//! `prompt_contents.prompt_id` sweeps every per-kind content row
//! inside the same transaction.

use objectiveai_sdk::cli::command::agents::message_queue::delete::{Request, Response};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::db;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let item = db::prompts::delete_by_id_async(ctx.filesystem.clone(), request.id)
        .await?
        .ok_or_else(|| {
            Error::Filesystem(crate::filesystem::Error::NotFound(format!(
                "message-queue prompt id {}",
                request.id
            )))
        })?;
    Ok(Response {
        id: request.id,
        agent_instance_hierarchy: item.agent_instance_hierarchy,
        agent_tag: item.agent_tag,
        key: item.key,
        enqueued_at: item.enqueued_at,
        content: item.content,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::message_queue::delete as sdk;
    use objectiveai_sdk::cli::command::agents::message_queue::delete::request_schema::{
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
    use objectiveai_sdk::cli::command::agents::message_queue::delete as sdk;
    use objectiveai_sdk::cli::command::agents::message_queue::delete::response_schema::{
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
