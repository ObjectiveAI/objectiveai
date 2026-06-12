//! `agents queue delete` — bare-naked handler. Atomically
//! drops one queued message by its `message_queue.id` and returns the
//! deleted row's metadata + reconstructed content body. Cascade on
//! `message_queue_contents.message_queue_id` sweeps every per-kind content row
//! inside the same transaction.

use objectiveai_sdk::cli::command::agents::queue::delete::{Request, Response};

use crate::context::Context;
use crate::db;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    use db::message_queue::DeleteOutcome;

    let outcome = db::message_queue::delete_by_id(
        ctx.db_client().await?,
        request.id,
        &ctx.config.agent_instance_hierarchy,
    )
    .await?;
    let item = match outcome {
        DeleteOutcome::Deleted(item) => item,
        DeleteOutcome::NotFound => {
            return Err(Error::Filesystem(crate::filesystem::Error::NotFound(
                format!("message-queue prompt id {}", request.id),
            )));
        }
        DeleteOutcome::Unauthorized {
            sender_agent_instance_hierarchy,
        } => {
            return Err(Error::QueueDeleteUnauthorized {
                id: request.id,
                sender_agent_instance_hierarchy,
                caller_agent_instance_hierarchy: ctx
                    .config
                    .agent_instance_hierarchy
                    .clone(),
            });
        }
    };
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
    use objectiveai_sdk::cli::command::agents::queue::delete as sdk;
    use objectiveai_sdk::cli::command::agents::queue::delete::request_schema::{
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
    use objectiveai_sdk::cli::command::agents::queue::delete as sdk;
    use objectiveai_sdk::cli::command::agents::queue::delete::response_schema::{
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
