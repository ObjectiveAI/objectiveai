//! A directory resource's tree, once.

use diverge_sdk::daemon::endpoints::resources::Kind;
use diverge_sdk::daemon::endpoints::resources::filetree::client::request;
use diverge_sdk::daemon::endpoints::resources::filetree::server::response::Frame;
use diverge_sdk::daemon::grant::resources::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::in_use;
use crate::content;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, resources};

/// Answer the tree and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `filetree` grant at all; `NotFound` for a
/// resource not held, or a path at which nothing is; `Forbidden` for a
/// resource the grants do not reach; `NotDirectory` for a file
/// resource, or a path at which a file is; else the tree of the
/// directory the path names, every directory in it with `changes`
/// false, since a resource never changes.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::resources::holds(&standing, Over::Filetree) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = resources::by_id(&mut conn, &frame.resource, false).await? else {
        return Ok(Frame::NotFound);
    };
    drop(conn);
    if !judge::resources::over(&standing, Over::Filetree, &record, in_use(&record)) {
        return Ok(Frame::Forbidden);
    }
    if record.kind == Kind::File {
        return Ok(Frame::NotDirectory);
    }
    if content::inside(&frame.path).is_err() {
        return Ok(Frame::NotFound);
    }
    let root = content::held(&daemon.resources, &record.id);
    Ok(match content::at(&root, &frame.path).await {
        None => Frame::NotFound,
        Some(content::Found::File(_)) => Frame::NotDirectory,
        Some(content::Found::Directory(dir)) => match content::tree(&dir).await {
            Ok(nodes) => Frame::Tree(nodes),
            Err(error) => Frame::Error(reply::failure(&error)),
        },
    })
}
