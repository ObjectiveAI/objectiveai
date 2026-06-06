//! `viewer kill` — terminate every running `objectiveai-viewer`
//! process. Idempotent: a count of zero is not an error.

use objectiveai_sdk::cli::command::viewer::kill::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    let killed = crate::spawn::kill_by_name("objectiveai-viewer");
    Ok(Response { killed })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::viewer::kill as sdk;
    use objectiveai_sdk::cli::command::viewer::kill::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::viewer::kill as sdk;
    use objectiveai_sdk::cli::command::viewer::kill::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
