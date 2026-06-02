//! `agents read all` — bare-naked streaming handler stub.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::read::all::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<ItemStream, Error> {
    todo!("agents read all execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::read::all as sdk;
    use objectiveai_sdk::cli::command::agents::read::all::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::read::all as sdk;
    use objectiveai_sdk::cli::command::agents::read::all::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
