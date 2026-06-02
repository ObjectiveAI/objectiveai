//! `tools list` — bare-naked streaming handler stub. Emits one
//! `ResponseItem` per installed tool.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::tools::list::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<ItemStream, Error> {
    todo!("tools list execute")
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
