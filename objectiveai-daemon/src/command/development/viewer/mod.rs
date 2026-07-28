//! `development viewer` tier — dispatch for the viewer-app source
//! registration. The handlers live one directory up
//! (`development/viewer_app_{set,get,delete}.rs`, named to stay clear
//! of the plugin-half `viewer_*` handlers) beside the registry slot
//! they share; this module is only the dispatch.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::development::viewer::{Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

use super::{viewer_app_delete, viewer_app_get, viewer_app_set};

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
    let stream: ItemStream = match request {
        Request::Set(req) => {
            let value = viewer_app_set::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Set(value)))
        }
        Request::SetRequestSchema(req) => {
            let value =
                viewer_app_set::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::SetRequestSchema(value)))
        }
        Request::SetResponseSchema(req) => {
            let value =
                viewer_app_set::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::SetResponseSchema(value)))
        }
        Request::Get(req) => {
            let value = viewer_app_get::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Get(value)))
        }
        Request::GetRequestSchema(req) => {
            let value =
                viewer_app_get::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::GetRequestSchema(value)))
        }
        Request::GetResponseSchema(req) => {
            let value =
                viewer_app_get::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::GetResponseSchema(value)))
        }
        Request::Delete(req) => {
            let value = viewer_app_delete::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Delete(value)))
        }
        Request::DeleteRequestSchema(req) => {
            let value = viewer_app_delete::request_schema::execute(global, scoped, req)
                .await?;
            once(Ok(ResponseItem::DeleteRequestSchema(value)))
        }
        Request::DeleteResponseSchema(req) => {
            let value =
                viewer_app_delete::response_schema::execute(global, scoped, req)
                    .await?;
            once(Ok(ResponseItem::DeleteResponseSchema(value)))
        }
    };
    Ok(stream)
}
