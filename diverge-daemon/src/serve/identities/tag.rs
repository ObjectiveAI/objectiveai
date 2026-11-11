//! Tagging one of the host's identities.

use diverge_sdk::daemon::endpoints::identities::tag::client::request;
use diverge_sdk::daemon::endpoints::identities::tag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::held;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, identities, tags};

/// Answer the tag and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `tag` grant at all; `NotFound` for an identity
/// not on the host; `Error` for files that could not be read;
/// `Forbidden` for an identity the grants do not reach, or a tag the
/// grant does not cover; else the tags on the identity — the daemon's,
/// kept by its issuer and subject — a tag held already held still, and an
/// empty list changing nothing.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::identities::holds_tagging(&standing, Tagging::Tag) {
        return Ok(Frame::Forbidden);
    }
    let found = match held(&mut tx, daemon).await? {
        Ok(identities) => identities
            .into_iter()
            .find(|identity| identity.issuer == frame.identity.issuer && identity.subject == frame.identity.subject),
        Err(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    let Some(identity) = found else {
        return Ok(Frame::NotFound);
    };
    if !judge::identities::tagging(&standing, Tagging::Tag, &identity, &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    identities::set_tags(&mut tx, &frame.identity, &tags::with(&identity.tags, &frame.tags)).await?;
    tx.commit().await?;
    Ok(Frame::Tagged)
}
