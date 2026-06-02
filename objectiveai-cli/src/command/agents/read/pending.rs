//! `agents read pending` — bare-naked streaming handler stub.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::read::pending::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let caller = ctx.config.agent_instance_hierarchy.clone();
    let fetches = request.agent_instance_hierarchies.into_iter().map(|sub| {
        let fs = ctx.filesystem.clone();
        let caller = caller.clone();
        async move {
            let spawned = format!("{caller}/{sub}");
            let items = fs.read_new_from_queue(&caller, &spawned).await?;
            let value = serde_json::to_value(items)
                .map_err(|e| Error::InlineDeserialize(e.into()))?;
            let items = serde_json::from_value(value)
                .map_err(|e| Error::InlineDeserialize(e.into()))?;
            Ok::<_, Error>(ResponseItem {
                agent_id: sub,
                items,
            })
        }
    });
    let results = futures::future::try_join_all(fetches).await?;
    Ok(Box::pin(futures::stream::iter(results.into_iter().map(Ok))))
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
        Ok(schemars::schema_for!(sdk::Response))
    }
}
