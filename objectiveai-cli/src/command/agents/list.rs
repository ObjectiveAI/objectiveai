//! `agents list` — enumerate agents from the local filesystem state.
//! Streams one `ResponseItem` (a `RemotePath`) per agent found under
//! `<state>/agents/<owner>/<repository>`.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::list::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::publish::Kind;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, _request: Request) -> Result<ItemStream, Error> {
    let stream = crate::retrieve::list(ctx, Kind::Agents).await;
    Ok(Box::pin(stream.map(Ok)))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::list as sdk;
    use objectiveai_sdk::cli::command::agents::list::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::list as sdk;
    use objectiveai_sdk::cli::command::agents::list::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
