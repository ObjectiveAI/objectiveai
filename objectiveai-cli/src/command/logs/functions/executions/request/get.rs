//! `logs functions executions request get` — read a stored log record from disk.

use objectiveai_sdk::cli::command::logs::functions::executions::request::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    Ok(ctx
        .filesystem
        .read_function_execution_request(&request.id)
        .await?)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::functions::executions::request::get as sdk;
    use objectiveai_sdk::cli::command::logs::functions::executions::request::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::functions::executions::request::get as sdk;
    use objectiveai_sdk::cli::command::logs::functions::executions::request::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
