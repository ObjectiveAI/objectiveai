//! Listing roles, and keeping the list.

use std::future::Future;

use diverge_sdk::daemon::endpoints::roles::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::roles::list::server::response::{Frame, Role};
use diverge_sdk::daemon::grant::roles::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Change, Source};
use crate::serve::reply;
use crate::store::{self, RoleId, roles};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every role any
/// `list` grant reaches that the request's filter lets through,
/// oldest created first, the first `count` of them, one frame each,
/// then the word that the list is whole — and from then on each role
/// added, changed or removed as the roles and the accounts holding
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
    if !judge::roles::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Listed {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::Roles, Kind::Accounts], frame.count, &source, |change| match change {
        Change::Added(role) => Frame::Added(role),
        Change::Changed(role) => Frame::Changed(role),
        Change::Removed(role) => Frame::Removed(role),
        Change::Listed => Frame::Listed,
    })
    .await
}

/// The roles as the caller may list them now.
struct Listed<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Listed<'_> {
    type Key = RoleId;
    type Item = Role;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(RoleId, Role)>, store::Error>> + Send {
        async move {
            let all = {
                let mut conn = self.daemon.store.acquire().await?;
                roles::all(&mut conn).await?
            };
            Ok(all
                .iter()
                .filter(|role| judge::roles::over(self.standing, Over::List, role))
                .filter(|role| filter::roles::test(self.filter, role))
                .map(|role| (role.id, role.report()))
                .collect())
        }
    }
}
