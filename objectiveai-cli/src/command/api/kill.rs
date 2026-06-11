//! `api kill` — terminate every running `objectiveai-api` process.
//! Idempotent: a count of zero is not an error.

use objectiveai_sdk::cli::command::api::kill::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    let killed = crate::spawn::kill_by_name("objectiveai-api");
    Ok(Response { killed })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::kill as sdk;
    use objectiveai_sdk::cli::command::api::kill::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::kill as sdk;
    use objectiveai_sdk::cli::command::api::kill::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
