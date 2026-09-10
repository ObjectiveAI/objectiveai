//! Joining a running tool container, from a scope and the directory.

use std::net::IpAddr;
use std::sync::Arc;

use super::Connect;
use crate::endpoints::containers::server::encoded::encoded;
use crate::decode::Decode as _;
use crate::endpoints::containers::server::family::{Family as _, Runs};
use crate::endpoints::containers::server::own::Own;
use crate::endpoints::containers::server::run::{Run, send};
use crate::endpoints::containers::server::serve;
use crate::endpoints::containers::tools::connect::client::request;
use crate::endpoints::containers::tools::run::server::handle::Tools;
use crate::server::answer::{Answer, answer};
use crate::server::container_client::ContainerClient;
use crate::server::directory::Directory;
use crate::server::scope_handle::ScopeHandle;
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
///    stream stays quiet, which is the connector attached.
/// 3. The proxy dialled at the run's address, and everything the
///    connector opens served — the tree, reads, writes whose content
///    this end asks the connector for, the five MCP exchanges — until
///    the connector disconnects, the run ends (the runner's stop, or
///    the container's own end, heard through the directory), or the
///    connector goes away. A connector's `Postgres` has no pair and
///    is finished with nothing.
/// 4. The tasks ended and the finish, bare. Nothing is stopped and
///    nothing released: the container is its runner's.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::server::handle::handle) reads every
/// request once to dispatch it, and hands the result here.
pub async fn handle(scope: ScopeHandle, request: request::Frame, address: IpAddr, directory: &Directory) {
    let scope = Arc::new(scope);
    let Some(attached) = directory.lookup(&request.0.id) else {
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

    let run = Arc::new(Run::new(Arc::clone(&scope), ContainerClient::new(attached.address)));
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
