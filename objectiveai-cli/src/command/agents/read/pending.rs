//! `agents read pending` — bare-naked streaming handler stub.

use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use objectiveai_sdk::cli::command::agents::read::pending::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let default_parent = ctx.config.agent_instance_hierarchy.clone();
    let fs = ctx.filesystem.clone();
    let stream = async_stream::stream! {
        let mut inflight = FuturesUnordered::new();
        for target in request.targets {
            let fs = fs.clone();
            // Per-target parent fallback: explicit `parent=` overrides
            // the cli's own position; otherwise fall back to ctx.
            let parent = target
                .parent_agent_instance_hierarchy
                .unwrap_or_else(|| default_parent.clone());
            let leaf = target.agent_instance;
            inflight.push(async move {
                let spawned = format!("{parent}/{leaf}");
                let items = fs.read_new_from_queue(&parent, &spawned).await?;
                Ok::<_, Error>(ResponseItem { agent_id: leaf, items })
            });
        }
        while let Some(result) = inflight.next().await {
            yield result;
        }
    };
    Ok(Box::pin(stream))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::read::pending as sdk;
    use objectiveai_sdk::cli::command::agents::read::pending::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::read::pending as sdk;
    use objectiveai_sdk::cli::command::agents::read::pending::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
