//! A database connection the container opened, carried to the caller.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded::encoded;
use super::super::family::Runs;
use super::super::own::Own;
use super::super::run::Run;
use crate::container_proxy::postgres;
use crate::container_proxy::requests::execute::Ask;
use crate::server::answer::{Answer, answer};

/// The pair, from this end.
///
/// The container's driver connected, and the proxy announced it on
/// `channel`. This end mints a connection id, opens its half toward
/// the caller — `Postgres { connection_id }`, whose answers are what
/// the DATABASE says — and opens the proxy's path, which is the
/// driver's socket both ways. Then two directions, each its own task:
///
/// - what the caller answers on this end's half goes into the socket,
///   and the caller's finish — or its empty finish at once, which is
///   declining to dial — closes it;
/// - what the driver writes goes into the pair's queue, where the
///   caller's half, opened by the caller as a channel of its own
///   quoting the id, takes it from — see [`serve::postgres`]. The
///   socket ending ends that half too.
///
/// [`serve::postgres`]: super::super::serve::postgres
pub(crate) async fn postgres<R: Runs>(run: Arc<Run>, ask: Ask) {
    let (connection_id, to_caller) = run.pairs.open();
    let Some(payload) = encoded(&R::Ask::from(Own::Postgres(connection_id))) else {
        run.pairs.forget(connection_id);
        return;
    };
    let mut half = run.scope.send_channel_request(&payload).await;
    let Ok((mut from_container, mut to_container)) = postgres::execute::execute(&run.client, ask.channel).await else {
        run.pairs.forget(connection_id);
        return;
    };

    let database = async move {
        while let Some(bytes) = half.response_receiver.recv().await {
            match answer(&bytes) {
                Some(Answer::Frame(payload)) => {
                    if to_container.send(payload).await.is_err() {
                        return;
                    }
                }
                Some(Answer::Finish) => break,
                None => {}
            }
        }
        let _ = to_container.finish().await;
    };
    run.spawn(database).await;

    while let Some(Ok(bytes)) = from_container.next().await {
        if to_caller.send(bytes).is_err() {
            break;
        }
    }
    // The socket is over. Dropping the sender is how the caller's
    // half hears it; a half never opened is forgotten with its queue.
    drop(to_caller);
    run.pairs.forget(connection_id);
}
