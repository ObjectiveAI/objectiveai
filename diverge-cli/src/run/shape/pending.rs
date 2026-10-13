//! One question, one answer that waits.

use diverge_sdk::daemon::client::pending;
use diverge_sdk::wire::client::handle::Handle;
use diverge_sdk::wire::decode::Decode;
use diverge_sdk::wire::encode::Encode;
use serde::Serialize;

use super::request;
use crate::run::{Emitter, Error};

/// The request put, its answer waited for and emitted: an agent's
/// message, answered when it lands.
pub async fn pending<Q, A, E>(handle: &Handle, bytes: &[u8], emitter: &mut Emitter) -> Result<(), Error>
where
    Q: Encode + for<'a> Decode<'a>,
    A: for<'a> Decode<'a, Error = E> + Serialize,
    E: std::error::Error + Send + Sync + 'static,
{
    let request: Q = request(bytes)?;
    let waiting = pending::execute(handle, &request).await.map_err(|error| Error::Exchange(error.to_string()))?;
    let answer: A = waiting.wait().await.map_err(|error| Error::Exchange(error.to_string()))?;
    emitter.emit(&answer).await?;
    Ok(())
}
