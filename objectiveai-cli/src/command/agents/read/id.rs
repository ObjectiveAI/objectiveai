//! `agents read id` — bare-naked handler stub.

use objectiveai_sdk::cli::command::agents::read::id::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("agents read id execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::read::id as sdk;
    use objectiveai_sdk::cli::command::agents::read::id::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::read::id as sdk;
    use objectiveai_sdk::cli::command::agents::read::id::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
