//! `config functions profiles favorites add` — bare-naked handler stub.

use objectiveai_sdk::cli::command::config::functions::profiles::favorites::add::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("config functions profiles favorites add execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::functions::profiles::favorites::add as sdk;
    use objectiveai_sdk::cli::command::config::functions::profiles::favorites::add::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::functions::profiles::favorites::add as sdk;
    use objectiveai_sdk::cli::command::config::functions::profiles::favorites::add::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
