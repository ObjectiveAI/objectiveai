//! Joining a running tool container, from a scope and the directory.

use std::net::IpAddr;
use std::sync::Arc;

use super::Connect;
use crate::provider::endpoints::containers::server::encoded::encoded;
use crate::wire::decode::Decode as _;
use crate::provider::endpoints::containers::server::family::{Family as _, Runs};
use crate::provider::endpoints::containers::server::own::Own;
use crate::provider::endpoints::containers::server::run::{Run, send};
use crate::provider::endpoints::containers::server::serve;
use crate::provider::endpoints::containers::tools::connect::client::request;
use crate::provider::endpoints::containers::tools::run::server::handle::Tools;
use crate::wire::server::answer::{Answer, answer};
use crate::provider::endpoints::containers::server::begin::Begin;
use crate::provider::server::directory::Directory;
use crate::wire::server::scope_handle::ScopeHandle;
use crate::shared::containers::authorize;
use crate::shared::error::Error;

/// Attach the connector to the container it named, if its runner
/// allows, and serve the connection until the connector leaves or the
/// container does.
///
/// In order:
///
/// 1. The container, found in the [`Directory`] by its id — or not,
///    which is the scope's one `Error`, then the finish.
/// 2. The runner asked, on the RUN scope: an
///    [`Authorize`](authorize::request::Authorize) carrying the
///    connector's address — this connection's peer, attested — and
///    the authorization it offered, asserted. One frame answers.
///    `Denied`, a finish with nothing, or a runner that is gone: the
///    scope's one `Error`, then the finish. `Authorized`: the main
///    stream stays quiet, which is the connector attached — and
///    entered in the directory as such, until it leaves, which is
///    what lets it transfer into this container from another.
/// 3. The proxy dialled at the run's address, and everything the
///    connector opens served — the tree, reads, writes whose content
///    this end asks the connector for, transfers, the five MCP
///    exchanges — until
///    the connector disconnects, the run ends (the runner's stop, or
///    the container's own end, heard through the directory), or the
///    connector goes away. A connector's `Postgres` has no pair and
///    is finished with nothing.
/// 4. The tasks ended and the finish, bare. Nothing is stopped and
///    nothing released: the container is its runner's.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::provider::server::handle::handle) reads every
/// request once to dispatch it, and hands the result here.
pub async fn handle(scope: ScopeHandle, request: request::Frame, client_identity: &str, address: IpAddr, directory: Arc<Directory>) {
    let scope = Arc::new(scope);
    let id = request.0.id;
    let Some(attached) = directory.lookup(&id) else {
        send(&scope, Connect::error(&missing())).await;
        scope.send_response_finish().await;
        return;
    };

    let authorize = authorize::request::Authorize {
        address,
        authorization: request.0.authorization,
    };
    if !authorized(&attached.scope, authorize).await {
        send(&scope, Connect::error(&denied())).await;
        scope.send_response_finish().await;
        return;
    }

    // An agent container is its runner's alone: it has no begin a
    // connector's exchanges could ride, and is not found.
    let Some(begin) = attached.begin else {
        send(&scope, Connect::error(&missing())).await;
        scope.send_response_finish().await;
        return;
    };
    let identity: Arc<str> = Arc::from(client_identity);
    if !directory.attach(&id, &identity) {
        // Gone between the lookup and now.
        send(&scope, Connect::error(&missing())).await;
        scope.send_response_finish().await;
        return;
    }
    let run = Arc::new(Run::new(
        Arc::clone(&scope),
        Arc::clone(&identity),
        Arc::clone(&directory),
        attached.proxy,
        Begin::Tools(begin),
        attached.ignore,
        attached.watched,
    ));
    let mut ended = attached.ended;
    let over = Arc::clone(&run);
    run.spawn(async move {
        // The run over, or the directory gone with the provider: the
        // connection ends either way.
        while !*ended.borrow_and_update() {
            if ended.changed().await.is_err() {
                break;
            }
        }
        over.over.notify_one();
    })
    .await;

    let _end = serve::serve::<Connect>(&run).await;
    directory.detach(&id, &identity);
    run.shutdown().await;
    scope.send_response_finish().await;
}

/// Ask the runner, on its scope, and read its one answer. The channel
/// is read to its finish, so its number comes back to the run.
async fn authorized(run_scope: &ScopeHandle, authorize: authorize::request::Authorize) -> bool {
    let ask: <Tools as Runs>::Ask<'_> = Own::Authorize(authorize).into();
    let Some(payload) = encoded(&ask) else {
        return false;
    };
    let mut channel = run_scope.send_channel_request(&payload).await;
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

/// No container runs under that id.
fn missing() -> Error {
    Error(serde_json::json!({ "kind": "missing" }))
}

/// The runner said no, or could not be asked.
fn denied() -> Error {
    Error(serde_json::json!({ "kind": "denied" }))
}
