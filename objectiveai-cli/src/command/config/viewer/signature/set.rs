//! `config viewer signature set` — bare-naked handler stub.

use objectiveai_sdk::cli::command::config::viewer::signature::set::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("config viewer signature set execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::viewer::signature::set as sdk;
    use objectiveai_sdk::cli::command::config::viewer::signature::set::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::viewer::signature::set as sdk;
    use objectiveai_sdk::cli::command::config::viewer::signature::set::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
