//! `agents queue read id` — resolve a `message_queue_contents.id` to its
//! typed payload via the per-kind content tables, returning a
//! `RichContentPart` directly.

use objectiveai_sdk::cli::command::agents::queue::open::{Request, Response};

use crate::context::Context;
use crate::db;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let row = db::message_queue::read_content(ctx.db_client().await?, request.id)
        .await?
        .ok_or_else(|| {
            Error::Filesystem(crate::filesystem::Error::NotFound(format!(
                "queue content id {}",
                request.id
            )))
        })?;
    // `Response` is a type alias for `RichContentPart`; the shared
    // helper does the variant mapping.
    Ok(db::message_queue::content_row_to_part(row))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::queue::open as sdk;
    use objectiveai_sdk::cli::command::agents::queue::open::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::queue::open as sdk;
    use objectiveai_sdk::cli::command::agents::queue::open::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
