//! Listing agent templates, and keeping the list.

use std::future::Future;

use diverge_sdk::daemon::grant::agents_templates::Over;
use diverge_sdk::daemon::endpoints::agents::templates::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::agents::templates::list::server::response::{Frame, Listed};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Change, Source};
use crate::serve::reply;
use crate::store::{self, agents_templates};
use super::in_use;

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every template
/// any `list` grant reaches that the request's filter lets through,
/// oldest first, the first `count` of them, one frame each, then the
/// word that the list is whole — and from then on each template added,
/// changed or removed as templates are made, tagged and deleted and as
/// the agents made from them come and go, until the client cancels.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::agents_templates::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Templates {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::AgentsTemplates, Kind::Agents], frame.count, &source, |change| match change {
        Change::Added(template) => Frame::Added(template),
        Change::Changed(template) => Frame::Changed(template),
        Change::Removed(template) => Frame::Removed(template),
        Change::Listed => Frame::Listed,
    })
    .await
}

/// The templates as the caller may list them now.
struct Templates<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Templates<'_> {
    type Key = String;
    type Item = Listed;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(String, Listed)>, store::Error>> + Send {
        async move {
            let (all, held) = {
                let mut conn = self.daemon.store.acquire().await?;
                (agents_templates::all(&mut conn).await?, store::in_use::agents_templates(&mut conn).await?)
            };
            Ok(all
                .iter()
                .map(|record| (record, in_use(&held, record)))
                .filter(|(record, in_use)| judge::agents_templates::over(self.standing, Over::List, record, *in_use))
                .filter(|(record, in_use)| filter::agents_templates::test(self.filter, record, *in_use))
                .map(|(record, _)| (record.id.clone(), record.report()))
                .collect())
        }
    }
}
