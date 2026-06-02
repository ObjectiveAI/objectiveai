//! `agents list available` — bare-naked streaming handler stub.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::list::available::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<ItemStream, Error> {
    todo!("agents list available execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::list::available as sdk;
    use objectiveai_sdk::cli::command::agents::list::available::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::list::available as sdk;
    use objectiveai_sdk::cli::command::agents::list::available::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
