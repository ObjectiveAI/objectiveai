//! Listing incoming credentials, and keeping the list.

use std::future::Future;

use diverge_sdk::daemon::endpoints::providers::incoming::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::providers::incoming::list::server::response::{Frame, Incoming};
use diverge_sdk::daemon::grant::providers_incoming::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Source};
use crate::serve::reply;
use crate::store::{self, providers_incoming};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every credential
/// any `list` grant reaches that the request's filter lets through,
/// oldest added first, the first `count` of them, one frame each,
/// then the word that the listing is whole — and from then on each
/// credential added, changed or removed as the records and the
/// connections change, until the client cancels. Which have a
/// provider connected is read afresh at every change.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::providers_incoming::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Listed {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::ProvidersIncoming], frame.count, &source, Frame::from)
    .await
}

/// The credentials as the caller may list them now.
struct Listed<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Listed<'_> {
    type Key = String;
    type Item = Incoming;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(String, Incoming)>, store::Error>> + Send {
        async move {
            let all = {
                let mut conn = self.daemon.store.acquire().await?;
                providers_incoming::all(&mut conn).await?
            };
            let mut listed = Vec::with_capacity(all.len());
            for credential in &all {
                let status = self.daemon.live.provider_status(&credential.provider()).await;
                let capacity = self.daemon.live.capacity(&credential.provider()).map(|frame| (*frame).clone());
                let connected = status.connected();
                if !judge::providers_incoming::over(self.standing, Over::List, credential, connected) {
                    continue;
                }
                if !filter::providers_incoming::test(self.filter, credential, connected) {
                    continue;
                }
                listed.push((credential.identity.clone(), credential.report(status, capacity)));
            }
            Ok(listed)
        }
    }
}
