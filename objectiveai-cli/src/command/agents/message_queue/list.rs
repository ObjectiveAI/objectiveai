//! `agents message-queue list` Ã¢â‚¬â€ bare-naked streaming handler. Mirrors
//! `agents list active`'s shape: optional positional parent
//! (defaults to ctx), `--jq`, async-stream over a `Vec`.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::message_queue::list::{Request, ResponseItem};

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
        let rows = match fs.queue_list(&parent).await {
            Ok(v) => v,
            Err(e) => {
                yield Err(Error::from(e));
                return;
            }
        };
        for item in rows {
            yield Ok(item);
        }
    };
    Ok(Box::pin(stream))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::message_queue::list as sdk;
    use objectiveai_sdk::cli::command::agents::message_queue::list::request_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::message_queue::list as sdk;
    use objectiveai_sdk::cli::command::agents::message_queue::list::response_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::ResponseItem),
        ))
    }
}
