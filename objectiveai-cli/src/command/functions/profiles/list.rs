//! `functions profiles list` — enumerate profiles from the local
//! filesystem state. Streams one `ResponseItem` (a `RemotePath`) per
//! profile found under `<state>/profiles/<owner>/<repository>`.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::functions::profiles::list::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::publish::Kind;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, _request: Request) -> Result<ItemStream, Error> {
    let stream = crate::retrieve::list(ctx, Kind::Profiles).await;
    Ok(Box::pin(stream.map(Ok)))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::profiles::list as sdk;
    use objectiveai_sdk::cli::command::functions::profiles::list::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::profiles::list as sdk;
    use objectiveai_sdk::cli::command::functions::profiles::list::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
