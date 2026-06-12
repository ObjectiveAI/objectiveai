//! `swarms list <source>` — enumerate swarms from a given source.
//! Streams one `ResponseItem` per swarm. Sources:
//!
//! - `Filesystem` / `Objectiveai` / `Mock` — delegates to the SDK's
//!   `list_swarms` HTTP endpoint with the matching `ListSwarmsSource`.
//! - `All` — filesystem + objectiveai, **de-duplicated**: ObjectiveAI
//!   items skip anything already covered by a filesystem item. The
//!   fetches run concurrently.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::swarms::list::{
    Request, RequestSource, ResponseItem,
};
use objectiveai_sdk::swarm::request::{ListSwarmsRequest, ListSwarmsSource};
use objectiveai_sdk::{RemotePath, swarm::list_swarms};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request.source {
        RequestSource::Filesystem => paths_to_stream(
            fetch_paths(ctx, ListSwarmsSource::Filesystem).await?,
        ),
        RequestSource::Objectiveai => paths_to_stream(
            fetch_paths(ctx, ListSwarmsSource::Objectiveai).await?,
        ),
        RequestSource::Mock => {
            paths_to_stream(fetch_paths(ctx, ListSwarmsSource::Mock).await?)
        }
        RequestSource::All => {
            // the legacy noted this should be joined into the same
            // future tree — porting that improvement is out of scope.
            let (fs_items, oai_items) = tokio::try_join!(
                fetch_paths(ctx, ListSwarmsSource::Filesystem),
                fetch_paths(ctx, ListSwarmsSource::Objectiveai),
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
    source: ListSwarmsSource,
) -> Result<Vec<RemotePath>, Error> {
    let resp = list_swarms(
        &ctx.http,
        ListSwarmsRequest {
            source: Some(source),
        },
    )
    .await?;
    Ok(resp.data)
}

/// Merge filesystem + objectiveai into a single de-duplicated list:
/// filesystem items first, then objectiveai items not already
/// covered by a filesystem item.
(
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
    use objectiveai_sdk::cli::command::swarms::list as sdk;
    use objectiveai_sdk::cli::command::swarms::list::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::swarms::list as sdk;
    use objectiveai_sdk::cli::command::swarms::list::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
