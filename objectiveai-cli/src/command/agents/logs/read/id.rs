//! `agents logs read id <id>` — resolve a `logs.messages."index"`
//! to its typed [`Response`] variant. The dispatch logic lives in
//! [`crate::db::logs::read_by_id`]; this handler is a thin wrapper.

use objectiveai_sdk::cli::command::agents::logs::read::id::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    crate::db::logs::read_by_id(ctx.db_client().await?, request.id)
        .await?
        .ok_or_else(|| {
            Error::Filesystem(crate::filesystem::Error::NotFound(format!(
                "logs.messages row at index {}",
                request.id
            )))
        })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::logs::read::id as sdk;
    use objectiveai_sdk::cli::command::agents::logs::read::id::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::logs::read::id as sdk;
    use objectiveai_sdk::cli::command::agents::logs::read::id::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
