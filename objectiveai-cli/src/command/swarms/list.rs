//! `swarms list <source>` — enumerate swarms from a given source.
//! Streams one `ResponseItem` per swarm. Sources:
//!
//! - `Favorites` — on-disk swarm favorites (no HTTP).
//! - `Filesystem` / `Objectiveai` / `Mock` — delegates to the SDK's
//!   `list_swarms` HTTP endpoint with the matching `ListSwarmsSource`.
//! - `All` — favorites + filesystem + objectiveai, **de-duplicated**:
//!   favorites first, filesystem items skipping anything already
//!   covered by a favorite, then ObjectiveAI items skipping anything
//!   covered by a favorite or a filesystem item. Filesystem and
//!   ObjectiveAI fetches run concurrently.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::swarms::list::{
    Request, RequestSource, ResponseFavorite, ResponseItem,
};
use objectiveai_sdk::swarm::request::{ListSwarmsRequest, ListSwarmsSource};
use objectiveai_sdk::{RemotePath, swarm::list_swarms};

use crate::command::list_helpers::favorite_matches_path;
use crate::context::Context;
use crate::error::Error;
use crate::filesystem::config::Favorite;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request.source {
        RequestSource::Favorites => {
            let favorites = read_favorites(ctx).await?;
            Box::pin(futures::stream::iter(
                favorites.into_iter().map(favorite_to_item).map(Ok),
            ))
        }
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
            // Match the legacy `crate::list::all`: pre-await favorites,
            // then run the two HTTP fetches concurrently. The TODO in
            // the legacy noted this should be joined into the same
            // future tree — porting that improvement is out of scope.
            let favorites = read_favorites(ctx).await?;
            let (fs_items, oai_items) = tokio::try_join!(
                fetch_paths(ctx, ListSwarmsSource::Filesystem),
                fetch_paths(ctx, ListSwarmsSource::Objectiveai),
            )?;
            let items = merge_all(&favorites, fs_items, oai_items);
            Box::pin(futures::stream::iter(items.into_iter().map(Ok)))
        }
    };
    Ok(stream)
}

async fn read_favorites(ctx: &Context) -> Result<Vec<Favorite>, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    Ok(config.swarms().get_favorites().to_vec())
}

fn favorite_to_item(f: Favorite) -> ResponseItem {
    ResponseItem::Favorite(ResponseFavorite {
        name: f.get_name().to_string(),
        note: f.get_note().to_string(),
        path: f.path,
    })
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

/// Merge favorites + filesystem + objectiveai into a single
/// deduplicated `Vec<ResponseItem>`. Mirrors `crate::list::all` from
/// the legacy CLI: favorites first, then filesystem items not covered
/// by any favorite, then objectiveai items not covered by a favorite
/// or an already-emitted filesystem item.
fn merge_all(
    favorites: &[Favorite],
    fs_items: Vec<RemotePath>,
    oai_items: Vec<RemotePath>,
) -> Vec<ResponseItem> {
    let mut items: Vec<ResponseItem> = Vec::new();

    for fav in favorites {
        items.push(favorite_to_item(fav.clone()));
    }

    for path in fs_items {
        if !favorites
            .iter()
            .any(|fav| favorite_matches_path(&fav.path, &path))
        {
            items.push(ResponseItem::Item(path));
        }
    }

    for path in oai_items {
        let dominated = favorites
            .iter()
            .any(|fav| favorite_matches_path(&fav.path, &path))
            || items.iter().any(|existing| {
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
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::swarms::list as sdk;
    use objectiveai_sdk::cli::command::swarms::list::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
