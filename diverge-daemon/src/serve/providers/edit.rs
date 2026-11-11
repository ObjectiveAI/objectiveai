//! Editing a provider's record.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::providers::Direction;
use diverge_sdk::daemon::endpoints::providers::edit::client::request;
use diverge_sdk::daemon::endpoints::providers::edit::server::response::Frame;
use diverge_sdk::daemon::grant::providers::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::daemon::lifecycle::Reason;

use crate::containers::lifecycle::End;
use crate::providers;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Answer the request and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// The rules:
///
/// 1. **Judged.** `Forbidden` with no `edit` grant at all; `NotFound`
///    for an identity no record has; `Forbidden` for a record the
///    grants do not reach.
/// 2. **Given replaces, absent leaves.** `announcement`,
///    `accept_daemons`, `messageable` and `description` given are
///    written over the record's. `announcement` given as `null` clears
///    it: the daemon announces itself there no more.
/// 3. **`ips`.** Given for an incoming record, they replace its `ips`
///    — `null`, any IP. Given for an outgoing record, as a value or as
///    `null`, the answer is the error, and nothing is written. The
///    direction itself, and an outgoing record's address, are no
///    member of the request.
/// 4. **Told.** The record written and the book read again, the lists
///    are told, and the answer is `Edited`: it waits on no provider.
/// 5. **The connection, closed.** When the provider's connection is
///    one it dialled in on, and the record no longer admits the IP it
///    dialled from, every run on it ends as `provider_edited` and the
///    connection is closed, through
///    [`reconcile`](providers::reconcile). No other edit closes a
///    connection, and an outgoing provider's is never closed by one.
/// 6. **The connection, kept.** Otherwise what the daemon holds open
///    on the connection is made what the record now says — its
///    announce, its accept, its listen, each opened or cancelled — by a
///    [`sync`](providers::sync), on a task of its own: the connection
///    stays, and every run on it goes on. A provider not connected is
///    synced when it connects. An edit of the `description` alone
///    changes nothing held.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(kept) = store::providers::by_identity(&mut tx, &frame.identity, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::providers::over(&standing, Over::Edit, &kept) {
        return Ok(Frame::Forbidden);
    }
    let mut record = kept;
    // Rule 2.
    if let Some(announcement) = frame.announcement {
        record.announcement = announcement;
    }
    if let Some(accept_daemons) = frame.accept_daemons {
        record.accept_daemons = accept_daemons;
    }
    if let Some(messageable) = frame.messageable {
        record.messageable = messageable;
    }
    if let Some(description) = frame.description {
        record.description = description;
    }
    // Rule 3.
    if let Some(ips) = frame.ips {
        match &mut record.direction {
            Direction::Incoming { ips: kept } => *kept = ips,
            Direction::Outgoing { .. } => {
                return Ok(Frame::Error(reply::failure(&"`ips` are an incoming provider's: this provider is outgoing, and has none to edit")));
            }
        }
    }
    store::providers::edit(&mut tx, &record).await?;
    tx.commit().await?;
    daemon.providers.reload(&daemon.store).await?;
    // Rule 5: read after the book, so a connection attached meanwhile
    // either is found here or found the new record itself — see
    // `attach`.
    let unadmitted = match daemon.live.provider_source(&record.identity).await {
        Some(providers::Source::Accepted(ip)) => !daemon.providers.admits(&record.identity, ip),
        Some(providers::Source::Dialled(_)) | None => false,
    };
    if unadmitted {
        providers::reconcile(daemon, &record.identity, Some(End::Unexpected(Reason::ProviderEdited))).await;
    } else {
        // Rule 6.
        tokio::spawn({
            let (daemon, identity) = (Arc::clone(daemon), record.identity.clone());
            async move { providers::sync(&daemon, &identity).await }
        });
    }
    // Rule 4.
    daemon.live.changed(Kind::Providers);
    Ok(Frame::Edited)
}
