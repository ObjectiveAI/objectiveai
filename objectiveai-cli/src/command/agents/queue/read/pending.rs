//! `agents queue read pending` — bare-naked streaming handler. Mirrors
//! `agents list active`'s shape: optional positional parent
//! (defaults to ctx), `--jq`, async-stream over a `Vec`.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::queue::read::pending::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<ItemStream, Error> {
    // The queue-walk used to scan a filesystem index alongside the
    // `prompts` table; the postgres-backed reader hasn't landed yet,
    // so the leaf returns the structured NotImplemented signal.
    let stream = async_stream::stream! {
        yield Err(Error::NotImplemented(
            "agents queue read pending (postgres reader pending)",
        ));
    };
    Ok(Box::pin(stream))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::queue::read::pending as sdk;
    use objectiveai_sdk::cli::command::agents::queue::read::pending::request_schema::{
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
    use objectiveai_sdk::cli::command::agents::queue::read::pending as sdk;
    use objectiveai_sdk::cli::command::agents::queue::read::pending::response_schema::{
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
