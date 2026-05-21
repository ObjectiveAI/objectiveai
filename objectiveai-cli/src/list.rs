use clap::Subcommand;
use objectiveai_sdk::cli::output::{Handle, Items, ListItem, Output, PairListItem};

#[derive(Subcommand)]
pub enum Source {
    /// List from the local filesystem
    Filesystem,
    /// List from favorites
    Favorites,
    /// List from ObjectiveAI
    Objectiveai,
    /// List from the Mock provider
    Mock,
    /// List from all sources
    All,
}

async fn emit_items(items: Vec<ListItem>, handle: &Handle) {
    Output::<Items<ListItem>>::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: Items { items } })
        .emit(handle)
        .await;
}

async fn emit_pair_items(items: Vec<PairListItem>, handle: &Handle) {
    Output::<Items<PairListItem>>::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: Items { items } })
        .emit(handle)
        .await;
}

/// Returns true if a favorite matches a remote path.
fn favorite_matches(fav: &objectiveai_sdk::filesystem::config::Favorite, path: &objectiveai_sdk::RemotePath) -> bool {
    favorite_matches_path(fav.path(), path)
}

/// Returns favorites only. No API call.
pub async fn favorites<F, Fut>(
    get_favorites: F,
    handle: &Handle,
) -> Result<(), crate::error::Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Vec<objectiveai_sdk::filesystem::config::Favorite>>,
{
    let items: Vec<ListItem> = get_favorites().await
        .into_iter()
        .map(ListItem::Favorite)
        .collect();
    emit_items(items, handle).await;
    Ok(())
}

/// Fetches from a single remote source via api::run.
pub async fn single<F>(
    list_remote: F,
    handle: &Handle,
) -> Result<(), crate::error::Error>
where
    F: FnOnce(objectiveai_sdk::HttpClient) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<objectiveai_sdk::RemotePath>, crate::error::Error>> + Send>> + Send + 'static,
{
    let handle = handle.clone();
    crate::api::run(move |http_client| Box::pin(async move {
        let items: Vec<ListItem> = list_remote(http_client).await?
            .into_iter()
            .map(ListItem::Item)
            .collect();
        emit_items(items, &handle).await;
        Ok(())
    })).await
}

/// Fetches from all sources with de-duplication via api::run.
///
/// 1. Favorites first
/// 2. Filesystem items that don't match any favorite
/// 3. Objectiveai items that don't match any favorite or filesystem item
pub async fn all<F, Fut, FsF, OaiF>(
    get_favorites: F,
    list_filesystem: FsF,
    list_objectiveai: OaiF,
    handle: &Handle,
) -> Result<(), crate::error::Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Vec<objectiveai_sdk::filesystem::config::Favorite>>,
    FsF: FnOnce(objectiveai_sdk::HttpClient) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<objectiveai_sdk::RemotePath>, crate::error::Error>> + Send>> + Send + 'static,
    OaiF: FnOnce(objectiveai_sdk::HttpClient) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<objectiveai_sdk::RemotePath>, crate::error::Error>> + Send>> + Send + 'static,
{
    // TODO: figure out how to not pre-await this (join with api::run concurrently)
    let favorites = get_favorites().await;
    let handle = handle.clone();
    crate::api::run(move |http_client| Box::pin(async move {

        let (fs_result, oai_result) = tokio::join!(
            list_filesystem(http_client.clone()),
            list_objectiveai(http_client),
        );
        let fs_items = fs_result?;
        let oai_items = oai_result?;

        let mut items: Vec<ListItem> = Vec::new();

        // Favorites first
        for fav in &favorites {
            items.push(ListItem::Favorite(fav.clone()));
        }

        // Filesystem items, skipping any that match a favorite
        for item in fs_items {
            if !favorites.iter().any(|fav| favorite_matches(fav, &item)) {
                items.push(ListItem::Item(item));
            }
        }

        // Objectiveai items, skipping any that match a favorite or filesystem item
        for item in oai_items {
            let dominated = favorites.iter().any(|fav| favorite_matches(fav, &item))
                || items.iter().any(|existing| match existing {
                    ListItem::Item(p) => p == &item,
                    _ => false,
                });
            if !dominated {
                items.push(ListItem::Item(item));
            }
        }

        emit_items(items, &handle).await;
        Ok(())
    })).await
}

// -- Pair variants (function-profile pairs) --

/// Compares a RemotePathCommitOptional against a RemotePath.
fn favorite_matches_path(
    fav_path: &objectiveai_sdk::RemotePathCommitOptional,
    path: &objectiveai_sdk::RemotePath,
) -> bool {
    match (fav_path, path) {
        (
            objectiveai_sdk::RemotePathCommitOptional::Github { owner: fo, repository: fr, commit: fc },
            objectiveai_sdk::RemotePath::Github { owner: po, repository: pr, commit: pc },
        ) => fo == po && fr == pr && fc.as_ref().is_none_or(|c| c == pc),
        (
            objectiveai_sdk::RemotePathCommitOptional::Filesystem { owner: fo, repository: fr, commit: fc },
            objectiveai_sdk::RemotePath::Filesystem { owner: po, repository: pr, commit: pc },
        ) => fo == po && fr == pr && fc.as_ref().is_none_or(|c| c == pc),
        (
            objectiveai_sdk::RemotePathCommitOptional::Mock { name: fn_ },
            objectiveai_sdk::RemotePath::Mock { name: pn },
        ) => fn_ == pn,
        _ => false,
    }
}

/// Returns true if a pair favorite matches a pair item (both function and profile match).
fn pair_favorite_matches(
    fav: &objectiveai_sdk::filesystem::config::PairFavorite,
    item: &objectiveai_sdk::functions::response::ListFunctionProfilePairItem,
) -> bool {
    favorite_matches_path(&fav.function, &item.function)
        && favorite_matches_path(&fav.profile, &item.profile)
}

/// Returns pair favorites only. No API call.
pub async fn pair_favorites<F, Fut>(
    get_favorites: F,
    handle: &Handle,
) -> Result<(), crate::error::Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Vec<objectiveai_sdk::filesystem::config::PairFavorite>>,
{
    let items: Vec<PairListItem> = get_favorites().await
        .into_iter()
        .map(PairListItem::Favorite)
        .collect();
    emit_pair_items(items, handle).await;
    Ok(())
}

/// Fetches pairs from ObjectiveAI via api::run.
pub async fn pair_single<F>(
    list_remote: F,
    handle: &Handle,
) -> Result<(), crate::error::Error>
where
    F: FnOnce(objectiveai_sdk::HttpClient) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<objectiveai_sdk::functions::response::ListFunctionProfilePairItem>, crate::error::Error>> + Send>> + Send + 'static,
{
    let handle = handle.clone();
    crate::api::run(move |http_client| Box::pin(async move {
        let items: Vec<PairListItem> = list_remote(http_client).await?
            .into_iter()
            .map(PairListItem::Item)
            .collect();
        emit_pair_items(items, &handle).await;
        Ok(())
    })).await
}

/// Fetches pairs from all sources with de-duplication via api::run.
/// Pairs only support ObjectiveAI source (no filesystem), so de-duplication
/// is: favorites first, then ObjectiveAI items not matching any favorite.
pub async fn pair_all<GF, GFut, F>(
    get_favorites: GF,
    list_objectiveai: F,
    handle: &Handle,
) -> Result<(), crate::error::Error>
where
    GF: FnOnce() -> GFut,
    GFut: std::future::Future<Output = Vec<objectiveai_sdk::filesystem::config::PairFavorite>>,
    F: FnOnce(objectiveai_sdk::HttpClient) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<objectiveai_sdk::functions::response::ListFunctionProfilePairItem>, crate::error::Error>> + Send>> + Send + 'static,
{
    // TODO: figure out how to not pre-await this (join with api::run concurrently)
    let favorites = get_favorites().await;
    let handle = handle.clone();
    crate::api::run(move |http_client| Box::pin(async move {
        let oai_items = list_objectiveai(http_client).await?;

        let mut items: Vec<PairListItem> = Vec::new();

        for fav in &favorites {
            items.push(PairListItem::Favorite(fav.clone()));
        }

        for item in oai_items {
            if !favorites.iter().any(|fav| pair_favorite_matches(fav, &item)) {
                items.push(PairListItem::Item(item));
            }
        }

        emit_pair_items(items, &handle).await;
        Ok(())
    })).await
}
