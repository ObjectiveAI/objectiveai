//! `functions get` — read a function definition by remote path.

use objectiveai_sdk::cli::command::functions::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let path = request.path;
    Ok(objectiveai_sdk::functions::get_function(&ctx.http, path).await?)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::get as sdk;
    use objectiveai_sdk::cli::command::functions::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::get as sdk;
    use objectiveai_sdk::cli::command::functions::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
