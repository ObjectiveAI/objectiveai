//! `tools list` — bare-naked streaming handler stub. Emits one
//! `ResponseItem` per installed tool.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::tools::list::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let offset = request.offset.unwrap_or(0);
    let limit = request.limit.unwrap_or(usize::MAX);
    let fs = ctx.filesystem.clone();
    let stream = async_stream::stream! {
        let manifests = fs.list_tools(offset, limit).await;
        for m in manifests {
            let value = match serde_json::to_value(&m) {
                Ok(v) => v,
                Err(e) => {
                    yield Err(Error::InlineDeserialize(e.into()));
                    return;
                }
            };
            match serde_json::from_value::<ResponseItem>(value) {
                Ok(item) => yield Ok(item),
                Err(e) => {
                    yield Err(Error::InlineDeserialize(e.into()));
                    return;
                }
            }
        }
    };
    Ok(Box::pin(stream))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::tools::list as sdk;
    use objectiveai_sdk::cli::command::tools::list::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::tools::list as sdk;
    use objectiveai_sdk::cli::command::tools::list::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
