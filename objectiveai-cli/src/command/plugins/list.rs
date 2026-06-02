//! `plugins list` — bare-naked streaming handler stub.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::plugins::list::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let offset = request.offset.unwrap_or(0);
    let limit = request.limit.unwrap_or(usize::MAX);
    let manifests = ctx.filesystem.list_plugins(offset, limit).await;
    let items: Result<Vec<ResponseItem>, Error> = manifests
        .into_iter()
        .map(|m| {
            // Field-identical on-disk shape — JSON round-trip avoids a
            // hand-coded per-field copy.
            let value = serde_json::to_value(&m)
                .map_err(|e| Error::InlineDeserialize(e.into()))?;
            serde_json::from_value(value).map_err(|e| Error::InlineDeserialize(e.into()))
        })
        .collect();
    Ok(Box::pin(futures::stream::iter(items?.into_iter().map(Ok))))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::plugins::list as sdk;
    use objectiveai_sdk::cli::command::plugins::list::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::plugins::list as sdk;
    use objectiveai_sdk::cli::command::plugins::list::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
