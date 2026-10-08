//! Creating a tool template.

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::tools::templates::create::client::request;
use diverge_sdk::daemon::endpoints::tools::templates::create::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, tools_templates};
use crate::store::hash;

/// Answer the create and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` without the `create` grant; else the template's id —
/// the hash of its own compact JSON — with `Created` when the daemon
/// made it, or made it again after a delete, and `Exists` when it was
/// there, the same id either way.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools_templates::create(&standing) {
        return Ok(Frame::Forbidden);
    }
    let id = hash::template_id(&frame.0).map_err(store::Error::Json)?;
    let new = tools_templates::New {
        id: id.clone(),
        template: frame.0,
        creator: Creator::Client(Client {
            identity: standing.identity.clone(),
        }),
    };
    let made = tools_templates::create(&mut tx, &new).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::ToolsTemplates);
    Ok(match made {
        tools_templates::Created::Created => Frame::Created(id),
        tools_templates::Created::Exists => Frame::Exists(id),
    })
}
