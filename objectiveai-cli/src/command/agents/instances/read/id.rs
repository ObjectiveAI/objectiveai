//! `agents read id <id>` — resolve a queue Id (SQL row id in the
//! `files` table) to its log file's typed content. The filesystem
//! dispatcher classifies the path, calls the matching typed read,
//! and wraps the value in the matching `Response` variant — the cli
//! leaf just returns it.

use objectiveai_sdk::cli::command::agents::instances::read::id::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    Ok(ctx.filesystem.read_file_by_id(request.id).await?)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::instances::read::id as sdk;
    use objectiveai_sdk::cli::command::agents::instances::read::id::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::instances::read::id as sdk;
    use objectiveai_sdk::cli::command::agents::instances::read::id::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
