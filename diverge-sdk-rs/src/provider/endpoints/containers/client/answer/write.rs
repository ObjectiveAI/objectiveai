//! The content of a write this end started, when the provider asks
//! for it.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::{Encoders, Writes};
use super::send::{Stop, finish, respond_bytes};
use crate::CHUNK_SIZE;
use crate::wire::client::handle::Handle;
use crate::shared::error::Error;

/// The pending content under `write_id` as `Body` frames of at most
/// the chunk size, then the finish; an `Error` frame, then the finish,
/// for content that failed part-way or an id nothing was started
/// under.
pub(crate) async fn write(
    handle: &Handle,
    scope: u32,
    channel: u32,
    write_id: u32,
    writes: Arc<Writes>,
    encoders: Encoders,
) -> Result<(), Stop> {
    let Some(mut content) = writes.take(write_id) else {
        let error = Error(serde_json::Value::String(format!("no write {write_id} is pending")));
        if let Some(bytes) = (encoders.write_error)(&error) {
            respond_bytes(handle, scope, channel, &bytes).await?;
        }
        return finish(handle, scope, channel).await;
    };
    while let Some(piece) = content.next().await {
        match piece {
            Ok(piece) => {
                for chunk in piece.chunks(CHUNK_SIZE) {
                    let bytes = (encoders.write_body)(chunk).ok_or(Stop)?;
                    respond_bytes(handle, scope, channel, &bytes).await?;
                }
            }
            Err(error) => {
                if let Some(bytes) = (encoders.write_error)(&error) {
                    respond_bytes(handle, scope, channel, &bytes).await?;
                }
                break;
            }
        }
    }
    finish(handle, scope, channel).await
}
