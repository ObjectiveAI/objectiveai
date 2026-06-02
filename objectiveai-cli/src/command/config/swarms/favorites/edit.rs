//! `config swarms favorites edit` — bare-naked handler stub.

use objectiveai_sdk::cli::command::config::swarms::favorites::edit::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("config swarms favorites edit execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::swarms::favorites::edit as sdk;
    use objectiveai_sdk::cli::command::config::swarms::favorites::edit::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::swarms::favorites::edit as sdk;
    use objectiveai_sdk::cli::command::config::swarms::favorites::edit::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
