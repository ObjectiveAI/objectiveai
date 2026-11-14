//! Getting one image in a Store of the daemon's.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::endpoints::images::get::client::request;
use diverge_sdk::daemon::endpoints::images::get::server::response::Frame;
use diverge_sdk::daemon::endpoints::images::list::server::response::Image;
use diverge_sdk::daemon::grant::images::Action;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Answer the get and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// The rules:
///
/// 1. `Forbidden` with no `get` grant over images at all.
/// 2. `NotFound` for a provider the daemon keeps no record of;
///    `Forbidden` for one no `get` grant reaches, by its identity or
///    its tags.
/// 3. **Read from what the daemon holds**, as a list is: the mirror of
///    its Store on the provider, once its listing is whole, which is
///    waited for as a volumes get waits. `Error` where there is no
///    mirror — the provider is not connected now, or keeps no Store —
///    or the listing failed, in its words. The provider is not
///    asked.
/// 4. `NotFound` where the listing holds no image of the digest; else
///    the image as a list reports it.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    // Rule 1.
    let Some(standing) = standing else {
        return Ok(Frame::Forbidden);
    };
    if !judge::images::holds(&standing, Action::Get) {
        return Ok(Frame::Forbidden);
    }
    // Rule 2.
    let records = daemon.providers.all();
    let Some(record) = records.iter().find(|record| record.identity == frame.provider) else {
        return Ok(Frame::NotFound);
    };
    if !judge::images::over(&standing, Action::Get, record) {
        return Ok(Frame::Forbidden);
    }
    // Rule 3.
    let Some(mirror) = daemon.live.store(&record.identity).await else {
        return Ok(Frame::Error(reply::failure(&"the provider is not connected now, or keeps no Store")));
    };
    if let Err(fail) = mirror.listed().await {
        return Ok(Frame::Error(fail.error()));
    }
    // Rule 4.
    let Some(image) = mirror.images().await.into_iter().find(|image| image.digest == frame.digest) else {
        return Ok(Frame::NotFound);
    };
    Ok(Frame::Found(Image {
        provider: record.identity.clone(),
        digest: image.digest,
        description: image.description,
        bytes: image.bytes,
        created: DateTime::<Utc>::from_timestamp(i64::try_from(image.created).unwrap_or(0), 0).unwrap_or_default(),
    }))
}
