//! Creating a tool template.

use diverge_sdk::daemon::endpoints::standalone_tools::templates::create::client::request;
use diverge_sdk::daemon::endpoints::standalone_tools::templates::create::server::response::Frame;
use std::sync::Arc;

use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::containers::{self, images};
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner;
use crate::serve::reply;
use crate::store::{self, tools_templates};
use crate::store::hash;

/// Answer the create and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// The rules, in order:
///
/// 1. `Forbidden` without the `create` grant.
/// 2. The `secrets` error for a declaration with an empty name or a
///    name twice.
/// 3. The `image` error for an image with no document the daemon can
///    read: found as [`images::document`] finds it, which asks a
///    provider the first time the digest is seen and leaves the
///    document on record. So every template the daemon holds names an
///    image whose document is on record.
/// 4. Else the template's id — the hash of its own compact JSON —
///    with `Created` when the daemon made it, or made it again after
///    a delete, and `Exists` when it was there, the same id either
///    way.
///
/// The grant is judged on a connection given back before the image is
/// asked after, so nothing of the store is held while a provider is
/// waited for.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        return Ok(Frame::Forbidden);
    };
    if !judge::standalone_tools_templates::create(&standing) {
        return Ok(Frame::Forbidden);
    }
    if let Err(what) = inner::declared(&frame.template.secrets) {
        return Ok(Frame::Error(reply::secrets(&what)));
    }
    if let Err(why) = images::document(daemon, &containers::image(&frame.template.image)).await? {
        return Ok(Frame::Error(reply::image(&why)));
    }
    let mut tx = daemon.store.begin().await?;
    let id = hash::template_id(&frame.template.hashed()).map_err(store::Error::Json)?;
    let new = tools_templates::New {
        id: id.clone(),
        template: frame.template,
        creator: standing.creator.clone(),
    };
    let made = tools_templates::create(&mut tx, &new).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::StandaloneToolsTemplates);
    Ok(match made {
        tools_templates::Created::Created => Frame::Created(id),
        tools_templates::Created::Exists => Frame::Exists(id),
    })
}
