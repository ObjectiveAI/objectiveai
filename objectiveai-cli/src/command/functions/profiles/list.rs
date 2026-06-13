//! `functions profiles list <source>` — enumerate profiles from a
//! given source. Streams one `ResponseItem` per profile. Sources:
//!
//! - `Filesystem` / `Objectiveai` / `Mock` — delegates to the SDK's
//!   `list_profiles` HTTP endpoint with the matching
//!   `ListProfilesSource`.
//! - `All` — filesystem + objectiveai, **de-duplicated**: ObjectiveAI
//!   items skip anything already covered by a filesystem item. The
//!   fetches run concurrently.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::functions::profiles::list::{
    Request, RequestSource, ResponseItem,
};
use objectiveai_sdk::functions::profiles::list_profiles;
use objectiveai_sdk::functions::profiles::request::{ListProfilesRequest, ListProfilesSource};
use objectiveai_sdk::RemotePath;

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request.source {
        RequestSource::Filesystem => paths_to_stream(
            fetch_paths(ctx, ListProfilesSource::Filesystem).await?,
        ),
        RequestSource::Objectiveai => paths_to_stream(
            fetch_paths(ctx, ListProfilesSource::Objectiveai).await?,
        ),
        RequestSource::Mock => {
            paths_to_stream(fetch_paths(ctx, ListProfilesSource::Mock).await?)
        }
        RequestSource::All => {
            let (fs_items, oai_items) = tokio::try_join!(
                fetch_paths(ctx, ListProfilesSource::Filesystem),
                fetch_paths(ctx, ListProfilesSource::Objectiveai),
            )?;
            let items = merge_all(fs_items, oai_items);
            Box::pin(futures::stream::iter(items.into_iter().map(Ok)))
        }
    };
    Ok(stream)
}

fn paths_to_stream(paths: Vec<RemotePath>) -> ItemStream {
    Box::pin(futures::stream::iter(
        paths.into_iter().map(Ok),
    ))
}

async fn fetch_paths(
    ctx: &Context,
    source: ListProfilesSource,
) -> Result<Vec<RemotePath>, Error> {
    let resp = list_profiles(
        ctx.api_client().await?,
        ListProfilesRequest {
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
    // `ResponseItem` is an alias of `RemotePath`, so items are paths.
    let mut items: Vec<ResponseItem> = fs_items;

    for path in oai_items {
        let dominated = items.iter().any(|existing| existing == &path);
        if !dominated {
            items.push(path);
        }
    }

    items
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::profiles::list as sdk;
    use objectiveai_sdk::cli::command::functions::profiles::list::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::profiles::list as sdk;
    use objectiveai_sdk::cli::command::functions::profiles::list::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
