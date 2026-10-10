//! Tagging an incoming credential.

use diverge_sdk::daemon::endpoints::providers::incoming::tag::client::request;
use diverge_sdk::daemon::endpoints::providers::incoming::tag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, providers_incoming, tags};

/// Answer the tag and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `tag` grant at all; `NotFound`; `Forbidden`
/// for a credential the grants do not reach, or a tag the grant does
/// not cover; else the tags on the credential, a tag held already
/// held still, and an empty list changing nothing.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_incoming::holds_tagging(&standing, Tagging::Tag) {
        return Ok(Frame::Forbidden);
    }
    let Some(credential) = providers_incoming::by_identity(&mut tx, &frame.identity, true).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_provider_connected(&credential.provider()).await;
    if !judge::providers_incoming::tagging(&standing, Tagging::Tag, &credential, connected, &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    providers_incoming::set_tags(&mut tx, credential.id, &tags::with(&credential.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::ProvidersIncoming);
    Ok(Frame::Tagged)
}
