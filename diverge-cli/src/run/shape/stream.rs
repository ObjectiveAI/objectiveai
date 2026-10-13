//! One question, many answers, and where to stop.

use diverge_sdk::daemon::client::stream;
use diverge_sdk::wire::client::handle::Handle;
use diverge_sdk::wire::decode::Decode;
use diverge_sdk::wire::encode::Encode;
use futures_util::StreamExt as _;
use serde::Serialize;
use serde_json::Value;

use super::{exchange, request};
use crate::run::{Emitter, Error};

/// Where a stream is cut: a frame, as JSON, after which the scope is
/// cancelled and the run ends. What keeps this command from holding a
/// scope open.
pub type Stop = fn(&Value) -> bool;

/// Never: the stream runs to its finish — a logs request, which the
/// daemon finishes at the log's end or, watching, at its `count`.
pub fn never(_: &Value) -> bool {
    false
}

/// At the `listed` frame: a list is whole, and the changes after it
/// are not this command's.
pub fn at_listed(value: &Value) -> bool {
    value.as_str() == Some("listed")
}

/// At the first snapshot: a filetree is whole, and the changes after
/// it are not this command's.
pub fn at_snapshot(value: &Value) -> bool {
    value.get("filetree").and_then(|frame| frame.get("type")).and_then(Value::as_str) == Some("snapshot")
}

/// The request put, every answer emitted as it comes, until the
/// finish or until `stop` says a frame was the last, when the scope
/// is cancelled.
pub async fn stream<Q, A, E>(handle: &Handle, bytes: &[u8], emitter: &mut Emitter, stop: Stop) -> Result<(), Error>
where
    Q: Encode + for<'a> Decode<'a>,
    A: for<'a> Decode<'a, Error = E> + Serialize + Unpin,
    E: std::error::Error + Send + Sync + Unpin + 'static,
{
    let request: Q = request(bytes)?;
    let (mut answers, cancel) = stream::execute_cancellable::<Q, A, E>(handle, &request, |_, payload| A::decode(payload))
        .await
        .map_err(exchange::open)?;
    while let Some(item) = answers.next().await {
        let answer = item.map_err(exchange::stream)?;
        let value = emitter.emit(&answer).await?;
        if stop(&value) {
            let _ = cancel.cancel().await;
            break;
        }
    }
    Ok(())
}
