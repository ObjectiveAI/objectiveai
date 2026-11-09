//! Getting one provider.

use diverge_sdk::daemon::endpoints::providers::get::client::request;
use diverge_sdk::daemon::endpoints::providers::get::server::response::Frame;
use diverge_sdk::daemon::endpoints::providers::list::server::response::Provider;
use diverge_sdk::daemon::grant::providers::Over;
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

/// `Forbidden` with no `get` grant at all; `NotFound` for an identity no
/// record has; `Forbidden` for a record the grants do not reach; else
/// the provider as a list reports it.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = daemon.providers.record(&frame.identity) else {
        return Ok(Frame::NotFound);
    };
    if !judge::providers::over(&standing, Over::Get, &record) {
        return Ok(Frame::Forbidden);
    }
    Ok(Frame::Found(report(daemon, &record).await))
}

/// The provider of `record` as a get and a list report it: the record,
/// where the daemon stands with it now, and its latest capacity, while
/// it is connected.
pub async fn report(daemon: &Daemon, record: &store::providers::Record) -> Provider {
    let status = daemon.live.provider_status(&record.identity).await;
    let capacity = if status.connected() { daemon.live.capacity(&record.identity).map(|frame| (*frame).clone()) } else { None };
    Provider {
        identity: record.identity.clone(),
        direction: record.direction.clone(),
        listable: record.listable,
        enabled: record.enabled,
        description: record.description.clone(),
        tags: record.tags.clone(),
        created: record.created,
        creator: record.creator.clone(),
        status,
        capacity,
    }
}
