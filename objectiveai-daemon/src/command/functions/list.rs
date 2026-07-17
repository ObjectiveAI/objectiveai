//! `functions list` — enumerate functions from the local filesystem
//! state. Streams one `ResponseItem` (a `RemotePath`) per function
//! found under `<state>/functions/<owner>/<repository>`.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::functions::list::{Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;
use crate::filesystem::publish::Kind;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, _request: Request) -> Result<ItemStream, Error> {
    let stream = crate::retrieve::list(global, scoped, Kind::Functions).await;
    Ok(Box::pin(stream.map(Ok)))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::list as sdk;
    use objectiveai_sdk::cli::command::functions::list::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::list as sdk;
    use objectiveai_sdk::cli::command::functions::list::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
