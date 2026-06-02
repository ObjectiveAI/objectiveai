//! `swarms publish` — bare-naked handler stub.

use objectiveai_sdk::cli::command::swarms::publish::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("swarms publish execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::swarms::publish as sdk;
    use objectiveai_sdk::cli::command::swarms::publish::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::swarms::publish as sdk;
    use objectiveai_sdk::cli::command::swarms::publish::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
