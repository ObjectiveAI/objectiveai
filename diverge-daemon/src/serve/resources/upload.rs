//! Uploading a resource.

use std::sync::Arc;

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::resources::upload::client::request;
use diverge_sdk::daemon::endpoints::resources::upload::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::content;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, resources};

/// Receive the upload, answer it, and finish the scope.
///
/// `Forbidden` without the `upload` grant, before any channel is
/// opened; else one content channel per file, opened by the daemon
/// and answered by the client, and when every one has finished the
/// bytes placed under their hash and the record held: `Uploaded(id)`
/// for bytes held anew, `Exists(id)` for bytes held already, whose
/// description is now this request's. A channel ended in an error or
/// unfinished, a path that is not one, or the store failing, is the
/// `Error`, and nothing is held.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let scope = Arc::new(scope);
    let answer = match serve(Arc::clone(&scope), frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// Why an upload did not hold: the store, or the content.
enum Failure {
    Store(store::Error),
    Content(content::Error),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Store(error) => write!(f, "{error}"),
            Failure::Content(error) => write!(f, "{error}"),
        }
    }
}

async fn serve(scope: Arc<ScopeHandle>, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, Failure> {
    let standing = {
        let mut conn = daemon.store.acquire().await.map_err(Failure::Store)?;
        Standing::of(&mut conn, who).await.map_err(Failure::Store)?
    };
    let Some(standing) = standing else {
        return Ok(Frame::Forbidden);
    };
    if !judge::resources::create(&standing) {
        return Ok(Frame::Forbidden);
    }
    let received = content::receive(scope, &frame, &daemon.incoming()).await.map_err(Failure::Content)?;
    let description = match &frame {
        request::Frame::File { description } | request::Frame::Directory { description, .. } => description.clone(),
    };
    let held = {
        let mut tx = daemon.store.begin().await.map_err(Failure::Store)?;
        let held = resources::hold(
            &mut tx,
            &resources::New {
                id: received.id.clone(),
                kind: received.kind,
                description,
                bytes: received.bytes,
                creator: Creator::Client(Client {
                    identity: standing.identity.clone(),
                }),
            },
        )
        .await
        .map_err(Failure::Store)?;
        if let Err(error) = content::place(&daemon.resources, &received.incoming, &received.id).await {
            content::discard(&received.incoming).await;
            return Err(Failure::Content(error));
        }
        tx.commit().await.map_err(store::Error::from).map_err(Failure::Store)?;
        daemon.live.changed(Kind::Resources);
        held
    };
    Ok(match held {
        resources::Held::Held => Frame::Uploaded(received.id),
        resources::Held::Exists => Frame::Exists(received.id),
    })
}
