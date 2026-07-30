//! `development plugins` tier — one subgroup, `mcp`.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::development::plugins::{Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub mod mcp;
pub mod viewer;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(
    global: &GlobalContext,
    scoped: &ScopedContext,
    request: Request,
) -> Result<ItemStream, Error> {
    use futures::StreamExt;
    let stream: ItemStream = match request {
        Request::Mcp(req) => {
            let inner = mcp::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Mcp)))
        }
        Request::Viewer(req) => {
            let inner = viewer::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Viewer)))
        }
    };
    Ok(stream)
}
