//! `agents list available <source>` — enumerate remote agents from a
//! given source. Streams one `ResponseItem` per agent. Sources:
//!
//! - `Filesystem` / `Objectiveai` / `Mock` — delegates to the SDK's
//!   `list_agents` HTTP endpoint with the matching `ListAgentsSource`.
//! - `All` — filesystem + objectiveai, **de-duplicated**: ObjectiveAI
//!   items skip anything already covered by a filesystem item. The
//!   fetches run concurrently.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::agent::list_agents;
use objectiveai_sdk::agent::request::{ListAgentsRequest, ListAgentsSource};
use objectiveai_sdk::cli::command::agents::list::{
    Request, RequestSource, ResponseItem,
};
use objectiveai_sdk::RemotePath;

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request.source {
        RequestSource::Filesystem => paths_to_stream(
            fetch_paths(ctx, ListAgentsSource::Filesystem).await?,
        ),
        RequestSource::Objectiveai => paths_to_stream(
            fetch_paths(ctx, ListAgentsSource::Objectiveai).await?,
        ),
        RequestSource::Mock => {
            paths_to_stream(fetch_paths(ctx, ListAgentsSource::Mock).await?)
        }
        RequestSource::All => {
            let (fs_items, oai_items) = tokio::try_join!(
                fetch_paths(ctx, ListAgentsSource::Filesystem),
                fetch_paths(ctx, ListAgentsSource::Objectiveai),
            )?;
            let items = merge_all(fs_items, oai_items);
            Box::pin(futures::stream::iter(items.into_iter().map(Ok)))
        }
    };
    Ok(stream)
}

fn paths_to_stream(paths: Vec<RemotePath>) -> ItemStream {
    Box::pin(futures::stream::iter(
        paths.into_iter().map(ResponseItem::Item).map(Ok),
    ))
}

async fn fetch_paths(
    ctx: &Context,
    source: ListAgentsSource,
) -> Result<Vec<RemotePath>, Error> {
    let resp = list_agents(
        ctx.api_client().await?,
        ListAgentsRequest {
            source: Some(source),
        },
    )
    .await?;
    Ok(resp.data)
}

fn merge_all(
    fs_items: Vec<RemotePath>,
    oai_items: Vec<RemotePath>,
) -> Vec<ResponseItem> {
    let mut items: Vec<ResponseItem> =
        fs_items.into_iter().map(ResponseItem::Item).collect();

    for path in oai_items {
        let dominated = items.iter().any(|existing| {
            matches!(existing, ResponseItem::Item(p) if p == &path)
        });
        if !dominated {
            items.push(ResponseItem::Item(path));
        }
    }

    items
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::list as sdk;
    use objectiveai_sdk::cli::command::agents::list::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::list as sdk;
    use objectiveai_sdk::cli::command::agents::list::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
