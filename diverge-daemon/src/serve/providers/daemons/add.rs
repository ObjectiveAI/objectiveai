//! Adding a daemon record.

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::providers::daemons::Link;
use diverge_sdk::daemon::endpoints::providers::daemons::add::client::request;
use diverge_sdk::daemon::endpoints::providers::daemons::add::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use sqlx::PgConnection;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner;
use crate::serve::reply;
use crate::store::{self, providers_daemons};

/// Answer the add and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` without the `add` grant; `NoProvider` for a link whose
/// provider is none on record; `Exists` for a name another record
/// has; else the record there, which a connected tool may name from
/// now on.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_daemons::create(&standing) {
        return Ok(Frame::Forbidden);
    }
    if !linked(&mut tx, &frame.links).await? {
        return Ok(Frame::NoProvider);
    }
    let new = providers_daemons::New {
        name: frame.name,
        mode: frame.mode,
        links: frame.links,
        creator: Creator::Client(Client {
            identity: standing.identity.clone(),
        }),
    };
    match providers_daemons::create(&mut tx, &new).await? {
        providers_daemons::Created::Created(_) => {}
        providers_daemons::Created::Exists => return Ok(Frame::Exists),
    }
    tx.commit().await?;
    daemon.live.changed(Kind::ProvidersDaemons);
    Ok(Frame::Added)
}

/// Whether every link's provider is on record, outgoing or incoming.
pub async fn linked(conn: &mut PgConnection, links: &[Link]) -> Result<bool, store::Error> {
    for link in links {
        if !inner::on_record(conn, &link.provider).await? {
            return Ok(false);
        }
    }
    Ok(true)
}
