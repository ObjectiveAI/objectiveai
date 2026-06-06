//! `functions executions` sub-tier.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::functions::executions::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod create;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    // `Request` / `ResponseItem` are plain aliases of the single
    // child's types — straight passthrough, no wrapping.
    create::execute(ctx, request).await
}
