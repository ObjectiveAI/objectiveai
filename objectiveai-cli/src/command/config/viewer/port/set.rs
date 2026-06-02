//! `config viewer port set` — bare-naked handler stub.

use objectiveai_sdk::cli::command::config::viewer::port::set::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("config viewer port set execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::viewer::port::set as sdk;
    use objectiveai_sdk::cli::command::config::viewer::port::set::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::viewer::port::set as sdk;
    use objectiveai_sdk::cli::command::config::viewer::port::set::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
