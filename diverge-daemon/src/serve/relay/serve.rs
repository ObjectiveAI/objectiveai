//! A relayed request judged, connected, and carried.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::ClientRequest;
use diverge_sdk::daemon::grant::providers_daemons::Over;
use diverge_sdk::shared::relay::Relay;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::conduit;
use crate::daemon::Daemon;
use crate::daemons;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::providers_daemons;

/// Serve `request` — decoded only for its name and its refusals —
/// on the daemon `relay` names, as the account it names, carrying
/// `bytes`, the request with `relay` taken out. `Forbidden`, in the
/// request's own words, with no standing, or with no `relay` grant
/// over the record as that account; the request's own `Error` for a
/// record or an account that is none on record, a store that cannot
/// be read, or a connection that could not be opened; else the
/// conduit, to the remote's finish — or, the connection ending at any
/// frame before that, the one [`LOST`] error, then the finish. An
/// invalid request has no words and is finished with nothing, as it
/// would have been.
pub async fn serve(scope: ScopeHandle, request: &ClientRequest<'_>, bytes: Vec<u8>, relay: Relay, who: Who, daemon: &Arc<Daemon>) {
    let scope = Arc::new(scope);
    let outcome = judge_and_connect(&relay, who, daemon).await;
    let handle = match outcome {
        Ok(handle) => handle,
        Err(Refusal::Forbidden) => {
            answer(&scope, request.forbidden()).await;
            return;
        }
        Err(Refusal::Error(error)) => {
            answer(&scope, request.error(reply::failure(&error))).await;
            return;
        }
    };
    match conduit::run(Arc::clone(&scope), &handle, &bytes).await {
        Ok(conduit::End::Finished) => {}
        Ok(conduit::End::Lost) | Err(_) => answer(&scope, request.error(reply::failure(&LOST))).await,
    }
}

/// The one error a relay ends with when the connection to the daemon
/// ends before the remote finishes the request — refused at the
/// first frame, or lost at any frame after: the same words either
/// way, as the request's own `error`, then the finish.
pub const LOST: &str = "the connection to the daemon ended before the request was answered";

/// Why a relay was not made.
enum Refusal {
    /// The caller may not act as that account on that daemon.
    Forbidden,
    /// Something else, in words.
    Error(String),
}

/// The caller judged and the connection taken or opened: the handle
/// every frame of the relay rides.
async fn judge_and_connect(relay: &Relay, who: Who, daemon: &Arc<Daemon>) -> Result<diverge_sdk::wire::client::handle::Handle, Refusal> {
    let record = {
        let mut conn = daemon.store.acquire().await.map_err(|error| Refusal::Error(error.to_string()))?;
        let Some(standing) = Standing::of(&mut conn, who).await.map_err(|error| Refusal::Error(error.to_string()))? else {
            return Err(Refusal::Forbidden);
        };
        if !judge::providers_daemons::holds(&standing, Over::Relay) {
            return Err(Refusal::Forbidden);
        }
        let Some(record) = providers_daemons::by_name(&mut conn, &relay.daemon, false)
            .await
            .map_err(|error| Refusal::Error(error.to_string()))?
        else {
            return Err(Refusal::Error("the daemon named is none on record".to_string()));
        };
        if record.account(&relay.account).is_none() {
            return Err(Refusal::Error("the daemon record has no account of the name".to_string()));
        }
        let connected = daemon.live.is_daemon_connected(&record.name).await;
        if !judge::providers_daemons::through(&standing, Over::Relay, &record, connected, &relay.account) {
            return Err(Refusal::Forbidden);
        }
        record
    };
    let peer = daemons::connect(daemon, &record, &relay.account).await.map_err(|fail| Refusal::Error(fail.to_string()))?;
    Ok(peer.connected.handle.clone())
}

/// The one answer and the finish; or the finish alone, for a request
/// with no words.
async fn answer(scope: &ScopeHandle, frame: Option<Vec<u8>>) {
    if let Some(frame) = frame {
        scope.send_response(&frame).await;
    }
    scope.send_response_finish().await;
}
