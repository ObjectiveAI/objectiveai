//! `plugins list` — bare-naked streaming handler stub.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::plugins::list::{Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(_global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let offset = request.offset.unwrap_or(0);
    let limit = request.limit.unwrap_or(usize::MAX);
    let fs = scoped.filesystem.clone();
    let stream = async_stream::stream! {
        let manifests = fs.list_plugins(offset, limit).await;
        for m in manifests {
            yield Ok(ResponseItem::from(m));
        }
    };
    Ok(Box::pin(stream))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::plugins::list as sdk;
    use objectiveai_sdk::cli::command::plugins::list::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::plugins::list as sdk;
    use objectiveai_sdk::cli::command::plugins::list::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
