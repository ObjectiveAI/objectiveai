//! Tagging a volume.

use diverge_sdk::daemon::endpoints::volumes::tag::client::request;
use diverge_sdk::daemon::endpoints::volumes::tag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Located, locate};
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, tags, volumes};

/// Answer the tag and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `tag` grant at all; `NotFound`; `Error` for a
/// provider that could not be asked; `Forbidden` for a volume the
/// grants do not reach, or a tag the grant does not cover; else the
/// tags on the volume — the daemon's, kept beside the provider's
/// listing — a tag held already held still, and an empty list
/// changing nothing.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::volumes::holds_tagging(&standing, Tagging::Tag) {
        return Ok(Frame::Forbidden);
    }
    let listed = match locate(&mut tx, daemon, &frame.volume).await? {
        Located::Volume(listed) => listed,
        Located::None => return Ok(Frame::NotFound),
        Located::Failed(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    if !judge::volumes::tagging(&standing, Tagging::Tag, &listed, &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    volumes::set_tags(&mut tx, &frame.volume, &tags::with(&listed.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Volumes);
    Ok(Frame::Tagged)
}
