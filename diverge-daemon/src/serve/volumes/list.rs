//! Listing volumes, provider by provider.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::endpoints::volumes::list::client::request;
use diverge_sdk::daemon::endpoints::volumes::list::server::response::Frame;
use diverge_sdk::daemon::grant::volumes::Over;
use diverge_sdk::daemon::reference;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use sqlx::PgConnection;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, filter};
use crate::serve::reply;
use crate::store::{self, providers_incoming, providers_outgoing};
use crate::volumes::{self, Listed};

/// Send every volume the grants and the filter admit, then finish
/// the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `list` grant at all; else the providers the
/// grants reach — every one on record when some grant is `any` or
/// names none, else the ones named — in record order, the connected
/// ones only, each asked for its listing, each volume judged and
/// filtered, at most `count` sent. A provider on record but not
/// connected contributes nothing; a connected one that could not be
/// asked is the error, after what was sent.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::volumes::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let named = judge::volumes::providers(&standing, Over::List);
    let providers: Vec<Identity> = on_record(&mut conn)
        .await?
        .into_iter()
        .filter(|identity| named.as_ref().is_none_or(|named| named.contains(identity)))
        .collect();
    let cap = frame.count.map_or(usize::MAX, |count| usize::try_from(count).unwrap_or(usize::MAX));
    let mut sent = 0;
    for identity in providers {
        if sent >= cap {
            break;
        }
        if daemon.live.provider(&identity).await.is_none() {
            continue;
        }
        let volumes = match volumes::list(daemon, &identity).await {
            Ok(volumes) => volumes,
            Err(fail) => {
                reply::reply(scope, &Frame::Error(reply::failure(&fail))).await;
                return Ok(());
            }
        };
        for volume in volumes {
            if sent >= cap {
                break;
            }
            let reference = reference::Volume {
                provider: identity.clone(),
                name: volume.name.clone(),
            };
            let (agents, tools) = volumes::mounters(&mut conn, &reference).await?;
            let listed = Listed {
                provider: identity.clone(),
                volume,
                agents,
                tools,
            };
            if !judge::volumes::over(&standing, Over::List, &listed) || !filter::volumes::test(&frame.filter, &listed) {
                continue;
            }
            reply::reply(scope, &Frame::Volume(listed.report())).await;
            sent += 1;
        }
    }
    Ok(())
}

/// Every provider on record, outgoing then incoming, each in its
/// record order.
async fn on_record(conn: &mut PgConnection) -> Result<Vec<Identity>, store::Error> {
    let mut identities: Vec<Identity> = providers_outgoing::all(conn).await?.iter().map(|outgoing| outgoing.identity()).collect();
    identities.extend(providers_incoming::all(conn).await?.iter().map(|incoming| incoming.provider()));
    Ok(identities)
}
