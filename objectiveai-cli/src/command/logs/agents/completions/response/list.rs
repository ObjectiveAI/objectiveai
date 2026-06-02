//! `logs agents completions response list` — paginate stored agent
//! completion logs and emit one `ResponseItem` per record. The
//! underlying filesystem read is one-shot (returns a `Vec` for the
//! requested `(offset, limit)` window); the cli leaf wraps that Vec
//! in `futures::stream::iter` so the leaf-side return matches the
//! streaming-leaf contract.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::logs::agents::completions::response::list::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let offset = request.offset.unwrap_or(0);
    let limit = request.limit.unwrap_or(usize::MAX);
    let items = ctx
        .filesystem
        .list_agent_completions(offset, limit)
        .await?;
    Ok(Box::pin(futures::stream::iter(items.into_iter().map(Ok))))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::list as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::list::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::list as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::list::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
