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
    let caller = ctx.config.agent_instance_hierarchy.clone();
    let fs = ctx.filesystem.clone();
    let stream = async_stream::stream! {
        let mut inflight = FuturesUnordered::new();
        for sub in request.agent_instance_hierarchies {
            let fs = fs.clone();
            let caller = caller.clone();
            inflight.push(async move {
                let spawned = format!("{caller}/{sub}");
                let items = fs.read_new_from_queue(&caller, &spawned).await?;
                let value = serde_json::to_value(items)
                    .map_err(|e| Error::InlineJson(e))?;
                let items = serde_json::from_value(value)
                    .map_err(|e| Error::InlineJson(e))?;
                Ok::<_, Error>(ResponseItem { agent_id: sub, items })
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
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::read::pending as sdk;
    use objectiveai_sdk::cli::command::agents::read::pending::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::ResponseItem))
    }
}
