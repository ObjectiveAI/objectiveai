//! `logs agents completions response messages refusal subscribe` — bare-naked handler stub.

use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::refusal::subscribe::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("logs agents completions response messages refusal subscribe execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::refusal::subscribe as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::refusal::subscribe::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::refusal::subscribe as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::refusal::subscribe::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
