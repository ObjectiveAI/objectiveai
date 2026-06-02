//! `agents list active` — bare-naked streaming handler stub.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::list::active::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let parent = request
        .parent_agent_instance_hierarchy
        .as_deref()
        .unwrap_or(&ctx.config.agent_instance_hierarchy);
    let actives = ctx.filesystem.list_active(parent).await?;
    let items: Result<Vec<ResponseItem>, Error> = actives
        .into_iter()
        .map(|a| {
            // Field-identical shape (`agent_id: String, last_log: u64`).
            let value =
                serde_json::to_value(&a).map_err(|e| Error::InlineDeserialize(e.into()))?;
            serde_json::from_value(value).map_err(|e| Error::InlineDeserialize(e.into()))
        })
        .collect();
    Ok(Box::pin(futures::stream::iter(items?.into_iter().map(Ok))))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::list::active as sdk;
    use objectiveai_sdk::cli::command::agents::list::active::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::list::active as sdk;
    use objectiveai_sdk::cli::command::agents::list::active::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
