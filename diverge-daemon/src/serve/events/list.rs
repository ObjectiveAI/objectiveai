//! Listing events, and keeping the list.

use std::collections::HashSet;

use diverge_sdk::daemon::endpoints::events::Event;
use diverge_sdk::daemon::endpoints::events::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::events::list::server::response::Frame;
use diverge_sdk::daemon::grant::events::Action;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use tokio::sync::broadcast;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Change};
use crate::serve::{files, reply};
use crate::store::{self, events};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant; else every event the
/// request's filter lets through, the latest `count` of them by id —
/// every one when `count` is absent, none when it is `0` — in id
/// order, one `Added` each, then the word that the list is whole; and
/// from then on, with no cap, every event the filter lets through
/// that is added or whose count is raised, until the client cancels.
///
/// Its own loop rather than [`stream::listing`], whose cap bounds the
/// listing throughout where this one's bounds the initial listing
/// alone. The word is subscribed to before the store is read, so a
/// change between the reading and the hearing is in one or the
/// other. The rules, at every `Events` word and whenever the listing
/// fell behind the words, for each difference between what was last
/// read and what is read now:
///
/// 1. An edge not read before: `Added`, and remembered as sent.
/// 2. An edge read before whose count differs, sent already:
///    `Changed`.
/// 3. An edge read before whose count differs, never sent — the
///    window left it out: `Added`, and remembered as sent, since the
///    client has not seen it.
/// 4. An edge removed cannot occur — nothing takes an edge away, and
///    the filter is fixed for the scope's life — so nothing is told
///    of one.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::events::holds(&standing, Action::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let mut words = daemon.live.changes();
    let mut known = read(daemon, &frame.filter).await?;
    let window = frame
        .count
        .map_or(known.len(), |count| usize::try_from(count).unwrap_or(usize::MAX).min(known.len()));
    let mut sent: HashSet<u64> = HashSet::new();
    for (id, event) in &known[known.len() - window..] {
        reply::reply(scope, &Frame::Added(event.clone())).await;
        sent.insert(*id);
    }
    reply::reply(scope, &Frame::Listed).await;
    loop {
        tokio::select! {
            word = words.recv() => {
                let again = match word {
                    Ok(kind) => kind == Kind::Events,
                    Err(broadcast::error::RecvError::Lagged(_)) => true,
                    Err(broadcast::error::RecvError::Closed) => return Ok(()),
                };
                if again {
                    let now = read(daemon, &frame.filter).await?;
                    for change in stream::diff(&known, &now) {
                        match change {
                            Change::Added(event) => {
                                sent.insert(event.id);
                                reply::reply(scope, &Frame::Added(event)).await;
                            }
                            Change::Changed(event) if sent.contains(&event.id) => {
                                reply::reply(scope, &Frame::Changed(event)).await;
                            }
                            Change::Changed(event) => {
                                sent.insert(event.id);
                                reply::reply(scope, &Frame::Added(event)).await;
                            }
                            Change::Removed(_) | Change::Listed => {}
                        }
                    }
                    known = now;
                }
            }
            () = files::cancelled(scope) => return Ok(()),
        }
    }
}

/// The events the filter lets through, by id ascending, each keyed by
/// its id.
async fn read(daemon: &Daemon, filter: &Filter) -> Result<Vec<(u64, Event)>, store::Error> {
    let all = {
        let mut conn = daemon.store.acquire().await?;
        events::all(&mut conn).await?
    };
    Ok(all
        .iter()
        .filter(|event| filter::events::test(filter, event))
        .map(|event| (event.id, event.report()))
        .collect())
}
