//! Asking a provider which tool containers a tenant runs.

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
use crate::serve::reply;
use crate::store;

/// Send the listing and finish the scope.
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
/// relayed — one container per response as the provider adds it,
/// the provider's error as the error — until the provider says the
/// listing is whole, on which the daemon stops the listing and
/// finishes: the listing as it stood, once, and nothing of what the
/// provider would send after. Nothing is narrowed and nothing is
/// transformed.
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
    while let Some(item) = listing.next().await {
        match item {
            Ok(response::Frame::Added(container)) if !stopped => reply::reply(scope, &Frame::Container(container)).await,
            // The listing as it stood is what this answers; what the
            // provider sends after the stop, until its finish, is
            // nobody's here.
            Ok(response::Frame::Added(_)) | Ok(response::Frame::Removed(_)) | Ok(response::Frame::Error(_)) => {}
            Ok(response::Frame::Listed) => {
                if !stopped {
                    stopped = true;
                    stop.stop().await;
                }
            }
            Err(ExecuteStreamError::Refused(error)) => {
                reply::reply(scope, &Frame::Error(error)).await;
                break;
            }
            Err(error) => {
                if !stopped {
                    reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
                }
                break;
            }
        }
    }
    Ok(())
}
