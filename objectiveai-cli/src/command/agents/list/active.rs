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
        .clone()
        .unwrap_or_else(|| ctx.config.agent_instance_hierarchy.clone());
    let fs = ctx.filesystem.clone();
    let stream = async_stream::stream! {
        let actives = match fs.list_active(&parent).await {
            Ok(v) => v,
            Err(e) => {
                yield Err(Error::from(e));
                return;
            }
        };
        for a in actives {
            // Field-identical shape (`agent_id: String, last_log: u64`).
            let value = match serde_json::to_value(&a) {
                Ok(v) => v,
                Err(e) => {
                    yield Err(Error::InlineJson(e));
                    return;
                }
            };
            match serde_json::from_value::<ResponseItem>(value) {
                Ok(item) => yield Ok(item),
                Err(e) => {
                    yield Err(Error::InlineJson(e));
                    return;
                }
            }
        }
    };
    Ok(Box::pin(stream))
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
        Ok(schemars::schema_for!(sdk::ResponseItem))
    }
}
