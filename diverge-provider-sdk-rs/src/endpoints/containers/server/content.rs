//! The content a caller mounts by identity, made present.

use std::sync::Arc;

use bytes::Bytes;
use futures_util::{StreamExt as _, future};

use super::encoded::encoded;
use super::family::Runs;
use super::own::Own;
use super::render;
use crate::decode::Decode as _;
use crate::server::answers::Answers;
use crate::server::identity_mount_manager::IdentityMountManager;
use crate::server::scope_handle::ScopeHandle;
use crate::shared::containers::fetch_directory;
use crate::shared::containers::request::Container;
use crate::shared::error::Error;

/// Every identity the request names, held by the store when this
/// returns [`Ok`].
///
/// What the store already holds — from an earlier run, of anyone's —
/// costs nothing; the rest is fetched from the caller, all of it at
/// once, on a channel per identity. A caller that does not hold what
/// it asked to mount, or whose content the store would not take, is
/// the run never coming up.
pub(crate) async fn ensure<R, S>(scope: &Arc<ScopeHandle>, store: &S, container: &Container) -> Result<(), Error>
where
    R: Runs,
    S: IdentityMountManager,
    S::Error: Into<Error>,
{
    let files = container
        .identity_file_mounts
        .iter()
        .map(|mount| file::<R, S>(scope, store, &mount.identity));
    let directories = container
        .identity_directory_mounts
        .iter()
        .map(|mount| directory::<R, S>(scope, store, &mount.identity));
    let (files, directories) = future::join(future::join_all(files), future::join_all(directories)).await;
    files.into_iter().chain(directories).collect()
}

/// One file: held, or fetched and stored.
async fn file<R, S>(scope: &ScopeHandle, store: &S, identity: &str) -> Result<(), Error>
where
    R: Runs,
    S: IdentityMountManager,
    S::Error: Into<Error>,
{
    if store.holds(identity).await {
        return Ok(());
    }
    let payload = encoded(&R::Ask::from(Own::FetchFile(identity))).ok_or_else(|| render::missing_content(identity))?;
    let Some(answers) = Answers::first(scope, &payload).await else {
        return Err(render::missing_content(identity));
    };
    // A piece that never came is a hash that will not match.
    let pieces = answers.filter_map(|item| future::ready(item.ok()));
    store.store_file(identity, pieces).await.map_err(Into::into)
}

/// One directory: held, or fetched and stored.
async fn directory<R, S>(scope: &ScopeHandle, store: &S, identity: &str) -> Result<(), Error>
where
    R: Runs,
    S: IdentityMountManager,
    S::Error: Into<Error>,
{
    if store.holds(identity).await {
        return Ok(());
    }
    let payload = encoded(&R::Ask::from(Own::FetchDirectory(identity))).ok_or_else(|| render::missing_content(identity))?;
    let Some(answers) = Answers::first(scope, &payload).await else {
        return Err(render::missing_content(identity));
    };
    let files = answers.filter_map(|item| future::ready(item.ok().and_then(piece)));
    store.store_directory(identity, files).await.map_err(Into::into)
}

/// One frame of a fetched directory: the file's path, and a piece of
/// its bytes. A frame that will not decode is skipped, which the
/// store's hash then catches.
fn piece(payload: Bytes) -> Option<(Vec<String>, Bytes)> {
    let fetch_directory::response::Frame { path, body } = fetch_directory::response::Frame::decode(&payload).ok()?;
    let body = payload.slice_ref(body);
    Some((path, body))
}
