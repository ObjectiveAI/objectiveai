//! One question, one answer.

use diverge_sdk::daemon::client::one_shot;
use diverge_sdk::wire::client::handle::Handle;
use diverge_sdk::wire::decode::Decode;
use diverge_sdk::wire::encode::Encode;
use serde::Serialize;

use super::{exchange, request};
use crate::run::{Emitter, Error};

/// The request put, its one answer emitted.
pub async fn one_shot<Q, A, E>(handle: &Handle, bytes: &[u8], emitter: &mut Emitter) -> Result<(), Error>
where
    Q: Encode + for<'a> Decode<'a>,
    A: for<'a> Decode<'a, Error = E> + Serialize,
    E: std::error::Error + Send + Sync + 'static,
{
    let request: Q = request(bytes)?;
    let answer: A = one_shot::execute(handle, &request).await.map_err(exchange::one_shot)?;
    emitter.emit(&answer).await?;
    Ok(())
}
