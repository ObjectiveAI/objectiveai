//! `functions profiles get` — bare-naked handler stub.

use objectiveai_sdk::cli::command::functions::profiles::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("functions profiles get execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::profiles::get as sdk;
    use objectiveai_sdk::cli::command::functions::profiles::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::profiles::get as sdk;
    use objectiveai_sdk::cli::command::functions::profiles::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
