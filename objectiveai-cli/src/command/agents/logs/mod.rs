//! `agents logs` — CLI-side dispatch for the logs subtree. One
//! sub-tier today: `read`. The SDK's `logs::Request` /
//! `logs::ResponseItem` are now type aliases for `read::*`, so
//! this tier just hands the request to `read::execute`.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::logs::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod read;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    read::execute(ctx, request).await
}
