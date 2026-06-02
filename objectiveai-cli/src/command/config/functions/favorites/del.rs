//! `config functions favorites del` — bare-naked handler stub.

use objectiveai_sdk::cli::command::config::functions::favorites::del::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("config functions favorites del execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::functions::favorites::del as sdk;
    use objectiveai_sdk::cli::command::config::functions::favorites::del::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::functions::favorites::del as sdk;
    use objectiveai_sdk::cli::command::config::functions::favorites::del::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
