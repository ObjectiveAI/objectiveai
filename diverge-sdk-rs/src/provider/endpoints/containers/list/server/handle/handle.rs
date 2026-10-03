//! Listing a running identity's containers, from a scope and the
//! directory.

use std::net::IpAddr;
use std::sync::Arc;

use tokio::task::JoinSet;

use crate::provider::endpoints::containers::agents::run::server::handle::Agents;
use crate::provider::endpoints::containers::list::client::request;
use crate::provider::endpoints::containers::list::server::response;
use crate::provider::endpoints::containers::server::encoded::encoded;
use crate::provider::endpoints::containers::server::family::Runs;
use crate::provider::endpoints::containers::server::own::Own;
use crate::provider::endpoints::containers::server::run::send;
use crate::provider::endpoints::containers::tools::run::server::handle::Tools;
use crate::provider::server::directory::{Directory, Running};
use crate::shared::containers::authorize;
use crate::wire::decode::Decode as _;
use crate::wire::server::answer::{Answer, answer};
use crate::wire::server::scope_handle::ScopeHandle;

/// Send the lister every container the identity runs whose runner
/// allows it, each as its runner answers, and finish.
///
/// In order:
///
/// 1. Every container the identity runs, found in the [`Directory`]
///    by its runner. None is a scope that finishes with nothing.
/// 2. Each runner asked, on its RUN scope, at once and all together:
///    an [`List`](authorize::request::AuthorizeList) carrying the lister's
///    address — this connection's peer, attested — and the identity
///    this connection was authorized under, attested too. One frame
///    answers each.
/// 3. `Authorized`: the container sent on this scope the moment the
///    answer lands, without waiting on any other runner. `Denied`, a
///    finish with nothing, a runner that is gone: nothing sent.
/// 4. The finish, once every ask has concluded.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::provider::server::handle::handle) reads every
/// request once to dispatch it, and hands the result here.
pub async fn handle(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &str,
    address: IpAddr,
    directory: Arc<Directory>,
) {
    let scope = Arc::new(scope);
    let request::Identity::Unbrokered { identity } = request.0;
    let lister = authorize::request::AuthorizeList {
        address,
        identity: client_identity.to_string(),
    };
    let mut asks = JoinSet::new();
    for running in directory.running_under(&identity) {
        let scope = Arc::clone(&scope);
        let lister = lister.clone();
        asks.spawn(async move {
            if authorized(&running, lister).await {
                let container = response::Container {
                    id: running.id,
                    family: running.family,
                };
                send(&scope, encoded(&response::Frame::Container(container))).await;
            }
        });
    }
    while asks.join_next().await.is_some() {}
    scope.send_response_finish().await;
}

/// Ask the runner, on its scope, and read its one answer. The channel
/// is read to its finish, so its number comes back to the run.
async fn authorized(running: &Running, lister: authorize::request::AuthorizeList) -> bool {
    let payload = match running.family {
        response::Family::Agent => {
            let ask: <Agents as Runs>::Ask<'_> = Own::AuthorizeList(lister).into();
            encoded(&ask)
        }
        response::Family::Tool => {
            let ask: <Tools as Runs>::Ask<'_> = Own::AuthorizeList(lister).into();
            encoded(&ask)
        }
    };
    let Some(payload) = payload else {
        return false;
    };
    let mut channel = running.scope.send_channel_request(&payload).await;
    let mut verdict = false;
    while let Some(bytes) = channel.response_receiver.recv().await {
        match answer(&bytes) {
            Some(Answer::Frame(payload)) => {
                if let Ok(authorize::response::Frame::Authorized) = authorize::response::Frame::decode(&payload) {
                    verdict = true;
                }
            }
            Some(Answer::Finish) => break,
            None => {}
        }
    }
    verdict
}
