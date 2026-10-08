//! Asking a provider which tool containers somebody runs, and
//! keeping the answer.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::endpoints::tools::list_for::client::request;
use diverge_sdk::daemon::endpoints::tools::list_for::server::response::Frame;
use diverge_sdk::daemon::grant::{providers_incoming, providers_outgoing};
use diverge_sdk::provider::endpoints::containers::tools::list_for::client::execute::{self, ExecuteStreamError};
use diverge_sdk::provider::endpoints::containers::tools::list_for::client::request as asked;
use diverge_sdk::provider::endpoints::containers::tools::list_for::server::response;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use futures_util::StreamExt as _;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{files, reply};
use crate::store;

/// Send the listing and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `list_for` grant over the provider's kind at
/// all, or for a provider the grants do not reach; `NoProvider` for
/// one not on record; the daemon's error for one on record that is
/// not connected now; else the provider protocol's own `list_for`
/// opened on the provider with the tenant, and what comes back
/// relayed as it comes — a container added, the word that the
/// listing is whole, a container removed — until the client cancels,
/// on which the provider's listing is stopped and the scope finishes
/// after the provider's; until the provider's scope ends; or until
/// the provider's error, relayed as the error. Nothing is narrowed
/// and nothing is transformed.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    let connected = daemon.live.is_provider_connected(&frame.provider).await;
    let allowed = match &frame.provider {
        Identity::Outgoing { address } => {
            if !judge::providers_outgoing::holds(&standing, providers_outgoing::Over::ListFor) {
                reply::reply(scope, &Frame::Forbidden).await;
                return Ok(());
            }
            let Some(provider) = store::providers_outgoing::by_address(&mut conn, address, false).await? else {
                reply::reply(scope, &Frame::NoProvider).await;
                return Ok(());
            };
            judge::providers_outgoing::over(&standing, providers_outgoing::Over::ListFor, &provider, connected)
        }
        Identity::IncomingUnbrokered { identity } => {
            if !judge::providers_incoming::holds(&standing, providers_incoming::Over::ListFor) {
                reply::reply(scope, &Frame::Forbidden).await;
                return Ok(());
            }
            let Some(credential) = store::providers_incoming::by_identity(&mut conn, identity, false).await? else {
                reply::reply(scope, &Frame::NoProvider).await;
                return Ok(());
            };
            judge::providers_incoming::over(&standing, providers_incoming::Over::ListFor, &credential, connected)
        }
    };
    drop(conn);
    if !allowed {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some(handle) = daemon.live.provider(&frame.provider).await else {
        reply::reply(scope, &Frame::Error(reply::failure(&"the provider is not connected now"))).await;
        return Ok(());
    };
    let (mut listing, stop) = match execute::execute(&handle, &asked::Frame(frame.tenant)).await {
        Ok(opened) => opened,
        Err(error) => {
            reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
            return Ok(());
        }
    };
    let mut stopped = false;
    loop {
        tokio::select! {
            item = listing.next() => match item {
                Some(Ok(frame)) if !stopped => match frame {
                    response::Frame::Added(container) => reply::reply(scope, &Frame::Added(container)).await,
                    response::Frame::Removed(container) => reply::reply(scope, &Frame::Removed(container)).await,
                    response::Frame::Listed => reply::reply(scope, &Frame::Listed).await,
                    response::Frame::Error(_) => {}
                },
                // After the stop, until the provider's finish, what the
                // provider sends is nobody's here.
                Some(Ok(_)) => {}
                Some(Err(ExecuteStreamError::Refused(error))) => {
                    if !stopped {
                        reply::reply(scope, &Frame::Error(error)).await;
                    }
                    return Ok(());
                }
                Some(Err(error)) => {
                    if !stopped {
                        reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
                    }
                    return Ok(());
                }
                None => return Ok(()),
            },
            () = files::cancelled(scope), if !stopped => {
                stopped = true;
                stop.stop().await;
            }
        }
    }
}
