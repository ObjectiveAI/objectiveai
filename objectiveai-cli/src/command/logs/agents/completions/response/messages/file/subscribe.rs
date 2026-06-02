//! `logs agents completions response messages file subscribe` — bare-naked handler stub.

use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::file::subscribe::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("logs agents completions response messages file subscribe execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::file::subscribe as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::file::subscribe::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::file::subscribe as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::file::subscribe::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
