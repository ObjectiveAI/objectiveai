//! `logs vector completions response clear` — clear a category of stored log records; returns the
//! number of records removed.

use objectiveai_sdk::cli::command::logs::vector::completions::response::clear::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let count = ctx.filesystem.clear_vector_completions().await?;
    Ok(Response { count })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::vector::completions::response::clear as sdk;
    use objectiveai_sdk::cli::command::logs::vector::completions::response::clear::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::vector::completions::response::clear as sdk;
    use objectiveai_sdk::cli::command::logs::vector::completions::response::clear::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
