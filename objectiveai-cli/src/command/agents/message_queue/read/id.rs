//! `agents message-queue read id` — resolve a `prompt_contents.id` to its
//! typed payload via the per-kind content tables, returning a
//! `RichContentPart` directly.

use objectiveai_sdk::cli::command::agents::message_queue::read::id::{Request, Response};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::db;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let row = db::prompts::read_content_async(ctx.filesystem.clone(), request.id)
        .await?
        .ok_or_else(|| {
            Error::Filesystem(crate::filesystem::Error::NotFound(format!(
                "queue content id {}",
                request.id
            )))
        })?;
    // `Response` is a type alias for `RichContentPart`; the shared
    // helper does the variant mapping.
    Ok(db::prompts::content_row_to_part(row))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::message_queue::read::id as sdk;
    use objectiveai_sdk::cli::command::agents::message_queue::read::id::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::message_queue::read::id as sdk;
    use objectiveai_sdk::cli::command::agents::message_queue::read::id::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
