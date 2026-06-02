//! `logs functions inventions recursive response subscribe` — wait (up to `timeout_ms`) for a log file to appear
//! or be modified, then read it. Timeout becomes
//! [`Error::Filesystem(LogSubscribeTimedOut)`].

use std::time::Duration;

use objectiveai_sdk::cli::command::logs::functions::inventions::recursive::response::subscribe::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    Ok(ctx
        .filesystem
        .subscribe_function_invention_recursive(
            &request.id,
            Duration::from_millis(request.timeout_ms),
            request.require_modification,
        )
        .await?)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::functions::inventions::recursive::response::subscribe as sdk;
    use objectiveai_sdk::cli::command::logs::functions::inventions::recursive::response::subscribe::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::functions::inventions::recursive::response::subscribe as sdk;
    use objectiveai_sdk::cli::command::logs::functions::inventions::recursive::response::subscribe::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
