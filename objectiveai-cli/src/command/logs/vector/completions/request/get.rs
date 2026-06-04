//! `logs vector completions request get` — read a stored log record from disk.

use objectiveai_sdk::cli::command::logs::vector::completions::request::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    Ok(ctx
        .filesystem
        .read_vector_completion_request(&request.id)
        .await?)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::vector::completions::request::get as sdk;
    use objectiveai_sdk::cli::command::logs::vector::completions::request::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::vector::completions::request::get as sdk;
    use objectiveai_sdk::cli::command::logs::vector::completions::request::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
