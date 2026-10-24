//! Listing vault keys, and keeping the list.

use std::future::Future;

use diverge_sdk::daemon::endpoints::vault::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::vault::list::server::response::{Frame, Key};
use diverge_sdk::daemon::grant::vault::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Source};
use crate::serve::reply;
use crate::store::{self, vault};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every key any
/// `list` grant reaches that the request's filter lets through,
/// oldest first, the first `count` of them, one frame each — its
/// slots by number and who holds each, never a value — then the word
/// that the list is whole; and from then on each key added, changed
/// or removed as the keys, their slots' holds and the records binding
/// them change, until the client cancels.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::vault::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Listed {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::Vault, Kind::Agents, Kind::StandaloneTools], frame.count, &source, Frame::from)
    .await
}

/// The keys as the caller may list them now.
struct Listed<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Listed<'_> {
    type Key = String;
    type Item = Key;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(String, Key)>, store::Error>> + Send {
        async move {
            let all = {
                let mut conn = self.daemon.store.acquire().await?;
                vault::all(&mut conn).await?
            };
            Ok(all
                .iter()
                .filter(|entry| judge::vault::over(self.standing, Over::List, entry))
                .filter(|entry| filter::vault::test(self.filter, entry))
                .map(|entry| (entry.key.clone(), entry.report()))
                .collect())
        }
    }
}
