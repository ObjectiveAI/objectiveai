//! `development plugins viewer` tier — the four leaves.
//!
//! The handlers live one directory up (`development/viewer_{create,
//! list,delete}.rs`) beside the `registry` and the stdin converge they
//! share; this module is only the dispatch.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::development::plugins::viewer::{Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

use super::super::{viewer_create as create, viewer_delete as delete, viewer_list as list};

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(
    global: &GlobalContext,
    scoped: &ScopedContext,
    request: Request,
) -> Result<ItemStream, Error> {
    use futures::StreamExt;
    let stream: ItemStream = match request {
        Request::Create(req) => {
            let value = create::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Create(value)))
        }
        Request::CreateRequestSchema(req) => {
            let value = create::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::CreateRequestSchema(value)))
        }
        Request::CreateResponseSchema(req) => {
            let value = create::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::CreateResponseSchema(value)))
        }
        Request::List(req) => {
            let inner = list::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::List)))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::ListRequestSchema(value)))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::ListResponseSchema(value)))
        }
        Request::Delete(req) => {
            let value = delete::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Delete(value)))
        }
        Request::DeleteRequestSchema(req) => {
            let value = delete::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::DeleteRequestSchema(value)))
        }
        Request::DeleteResponseSchema(req) => {
            let value = delete::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::DeleteResponseSchema(value)))
        }
    };
    Ok(stream)
}
