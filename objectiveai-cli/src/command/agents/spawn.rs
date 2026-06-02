//! `agents spawn` — bare-naked chunk-or-id streaming handler stub.
//! Single `execute` decides streaming-chunks vs single-id internally
//! based on `request.dangerous_advanced.stream`.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::spawn::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<ItemStream, Error> {
    todo!("agents spawn execute (chunk-or-id)")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::spawn as sdk;
    use objectiveai_sdk::cli::command::agents::spawn::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::spawn as sdk;
    use objectiveai_sdk::cli::command::agents::spawn::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
