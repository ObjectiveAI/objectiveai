//! `logs vector` sub-tier.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::logs::vector::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod completions;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    // `Request` / `ResponseItem` are plain aliases of the single
    // child's types — straight passthrough, no wrapping.
    completions::execute(ctx, request).await
}
