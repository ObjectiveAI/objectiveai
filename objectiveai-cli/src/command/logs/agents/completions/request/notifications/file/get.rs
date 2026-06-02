//! `logs agents completions request notifications file get` — bare-naked handler stub.

use objectiveai_sdk::cli::command::logs::agents::completions::request::notifications::file::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("logs agents completions request notifications file get execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::request::notifications::file::get as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::request::notifications::file::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::request::notifications::file::get as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::request::notifications::file::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
