//! One question, one answer that waits.

use diverge_sdk::daemon::client::pending;
use diverge_sdk::wire::client::handle::Handle;
use diverge_sdk::wire::decode::Decode;
use diverge_sdk::wire::encode::Encode;
use serde::Serialize;

use super::exchange;
use crate::run::{Emitter, Error};

/// The request put, its answer waited for and emitted: an agent's
/// message, answered when it lands.
pub async fn pending<Q, A, E>(
    handle: &Handle,
    request: &Q,
    emitter: &mut Emitter,
) -> Result<(), Error>
where
    Q: Encode,
    A: for<'a> Decode<'a, Error = E> + Serialize,
    E: std::error::Error + Send + Sync + 'static,
{
    let waiting = pending::execute(handle, request)
        .await
        .map_err(exchange::open)?;
    let answer: A = waiting.wait().await.map_err(exchange::one_shot)?;
    emitter.emit(&answer).await?;
    Ok(())
}
