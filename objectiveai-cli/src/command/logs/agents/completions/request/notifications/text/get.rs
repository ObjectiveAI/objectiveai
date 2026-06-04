//! `logs agents completions request notifications text get` — read a stored media payload from disk.

use objectiveai_sdk::cli::command::logs::agents::completions::request::notifications::text::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    Ok(ctx
        .filesystem
        .read_agent_completion_notification_text(
            &request.response_id,
            request.index,
            request.media_index,
        )
        .await?)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::request::notifications::text::get as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::request::notifications::text::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::request::notifications::text::get as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::request::notifications::text::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
