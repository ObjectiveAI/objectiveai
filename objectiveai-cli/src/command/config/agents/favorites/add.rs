//! `config agents favorites add` — bare-naked handler stub.

use objectiveai_sdk::cli::command::config::agents::favorites::add::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("config agents favorites add execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::agents::favorites::add as sdk;
    use objectiveai_sdk::cli::command::config::agents::favorites::add::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::agents::favorites::add as sdk;
    use objectiveai_sdk::cli::command::config::agents::favorites::add::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
