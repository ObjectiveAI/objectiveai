//! Reading an image's DIVERGE.md.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::images::info::client::request;
use diverge_sdk::daemon::endpoints::images::info::server::response::Frame;
use diverge_sdk::daemon::grant::images::Action;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::containers::images;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Answer the image's info and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(&scope, frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `info` grant over images; else the answer
/// [`images::info`] reaches.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        return Ok(Frame::Forbidden);
    };
    if !judge::images::holds(&standing, Action::Info) {
        return Ok(Frame::Forbidden);
    }
    scope.allowed().await;
    images::info(daemon, &frame.image).await
}
