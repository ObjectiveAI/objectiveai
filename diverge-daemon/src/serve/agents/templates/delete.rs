//! Deleting a agent template.

use diverge_sdk::daemon::grant::agents_templates::Over;
use diverge_sdk::daemon::endpoints::agents::templates::delete::client::request;
use diverge_sdk::daemon::endpoints::agents::templates::delete::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::database::{self, Owner};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, agents_templates};
use super::in_use;

/// Answer the delete and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `delete` grant at all; `NotFound`; `Forbidden`
/// for a template the grants do not reach; `InUse` while some agent
/// was made from it; else the template deleted — the compartment of
/// every `per_agent_template` dependency its agents ever had dropped
/// with its every table, its row kept, so one made anew is this one
/// again, with fresh compartments.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::agents_templates::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = agents_templates::by_id(&mut tx, &frame.id, true).await? else {
        return Ok(Frame::NotFound);
    };
    let held = store::in_use::agents_templates(&mut tx).await?;
    let used = in_use(&held, &record);
    if !judge::agents_templates::over(&standing, Over::Delete, &record, used) {
        return Ok(Frame::Forbidden);
    }
    if used {
        return Ok(Frame::InUse);
    }
    database::provision::sweep(
        &mut tx,
        &database::prefix_of(&Owner::AgentTemplate {
            template: record.id.clone(),
        }),
    )
    .await?;
    agents_templates::delete(&mut tx, &record.id).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::AgentsTemplates);
    Ok(Frame::Deleted)
}
