//! Listing providers, and keeping the list.

use std::future::Future;

use diverge_sdk::daemon::endpoints::providers::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::providers::list::server::response::{Frame, Provider};
use diverge_sdk::daemon::grant::providers::Action;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::get::report;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Source};
use crate::serve::reply;
use crate::store;

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant; else every provider the
/// configuration accepts that the request's filter lets through, in
/// the configuration's order, the first `count` of them, one frame
/// each, then the word that the listing is whole — and from then on
/// each provider changed as the connections change, until the client
/// cancels.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::providers::holds(&standing, Action::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Listed {
        daemon,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::Providers], frame.count, &source, Frame::from).await
}

/// The providers as the filter lets them through now.
struct Listed<'a> {
    daemon: &'a Daemon,
    filter: &'a Filter,
}

impl Source for Listed<'_> {
    type Key = Identity;
    type Item = Provider;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(Identity, Provider)>, store::Error>> + Send {
        async move {
            let mut listed = Vec::with_capacity(self.daemon.providers.accepted.len());
            for identity in &self.daemon.providers.accepted {
                let provider = report(self.daemon, identity).await;
                if filter::providers::test(self.filter, identity, provider.status.connected()) {
                    listed.push((identity.clone(), provider));
                }
            }
            Ok(listed)
        }
    }
}
