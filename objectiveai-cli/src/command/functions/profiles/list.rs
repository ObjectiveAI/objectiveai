//! `functions profiles list <source>` — enumerate profiles from a
//! given source. Streams one `ResponseItem` per profile. Sources:
//!
//! - `Favorites` — on-disk profile favorites (no HTTP).
//! - `Filesystem` / `Objectiveai` / `Mock` — delegates to the SDK's
//!   `list_profiles` HTTP endpoint with the matching
//!   `ListProfilesSource`.
//! - `All` — favorites + filesystem + objectiveai, **de-duplicated**:
//!   favorites first, filesystem items skipping anything already
//!   covered by a favorite, then ObjectiveAI items skipping anything
//!   covered by a favorite or a filesystem item. Filesystem and
//!   ObjectiveAI fetches run concurrently.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::functions::profiles::list::{
    Request, RequestSource, ResponseFavorite, ResponseItem,
};
use objectiveai_sdk::functions::profiles::list_profiles;
use objectiveai_sdk::functions::profiles::request::{ListProfilesRequest, ListProfilesSource};
use objectiveai_sdk::RemotePath;

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
            fetch_paths(ctx, ListProfilesSource::Filesystem).await?,
        ),
        RequestSource::Objectiveai => paths_to_stream(
            fetch_paths(ctx, ListProfilesSource::Objectiveai).await?,
        ),
        RequestSource::Mock => {
            paths_to_stream(fetch_paths(ctx, ListProfilesSource::Mock).await?)
        }
        RequestSource::All => {
            let favorites = read_favorites(ctx).await?;
            let (fs_items, oai_items) = tokio::try_join!(
                fetch_paths(ctx, ListProfilesSource::Filesystem),
                fetch_paths(ctx, ListProfilesSource::Objectiveai),
            )?;
            let items = merge_all(&favorites, fs_items, oai_items);
            Box::pin(futures::stream::iter(items.into_iter().map(Ok)))
        }
    };
    Ok(stream)
}

async fn read_favorites(ctx: &Context) -> Result<Vec<Favorite>, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    Ok(config.functions().profiles().get_favorites().to_vec())
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
    source: ListProfilesSource,
) -> Result<Vec<RemotePath>, Error> {
    let resp = list_profiles(
        &ctx.http,
        ListProfilesRequest {
            source: Some(source),
        },
    )
    .await?;
    Ok(resp.data)
}

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
