//! Listing accounts, and keeping the list.

use std::future::Future;

use diverge_sdk::daemon::endpoints::accounts::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::accounts::list::server::response::{Account, Frame};
use diverge_sdk::daemon::grant::accounts::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Change, Source};
use crate::serve::reply;
use crate::store::{self, AccountId, accounts};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every account
/// any `list` grant reaches that the request's filter lets through,
/// oldest created first, the first `count` of them, one frame each,
/// then the word that the list is whole — and from then on each
/// account added, changed or removed as the records, the roles and the
/// connections change, until the client cancels. Whether a client is
/// connected is read afresh at every change.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::accounts::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Listed {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::Accounts, Kind::Roles], frame.count, &source, |change| match change {
        Change::Added(account) => Frame::Added(account),
        Change::Changed(account) => Frame::Changed(account),
        Change::Removed(account) => Frame::Removed(account),
        Change::Listed => Frame::Listed,
    })
    .await
}

/// The accounts as the caller may list them now.
struct Listed<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Listed<'_> {
    type Key = AccountId;
    type Item = Account;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(AccountId, Account)>, store::Error>> + Send {
        async move {
            let all = {
                let mut conn = self.daemon.store.acquire().await?;
                accounts::all(&mut conn).await?
            };
            let connected = self.daemon.live.connected().await;
            Ok(all
                .iter()
                .map(|account| (account, connected.contains(&account.id)))
                .filter(|(account, connected)| judge::accounts::over(self.standing, Over::List, account, *connected))
                .filter(|(account, connected)| filter::accounts::test(self.filter, account, *connected))
                .map(|(account, connected)| (account.id, account.report(connected)))
                .collect())
        }
    }
}
