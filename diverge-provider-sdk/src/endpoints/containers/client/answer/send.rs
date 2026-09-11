//! Sending an answer's frames, and its finish.

use super::super::encoded;
use crate::CHUNK_SIZE;
use crate::client::handle::Handle;
use crate::encode::Encode;

/// An answer that ended before its finish: the frame would not
/// encode, or the write failed. Nothing to report — the provider reads
/// the channel's end — so nothing is carried.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Stop;

/// One channel response.
pub(crate) async fn respond<T: Encode>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    frame: &T,
) -> Result<(), Stop> {
    let bytes = encoded(frame).ok_or(Stop)?;
    respond_bytes(handle, scope, channel, &bytes).await
}

/// One channel response, already encoded.
pub(crate) async fn respond_bytes(handle: &Handle, scope: u32, channel: u32, bytes: &[u8]) -> Result<(), Stop> {
    handle
        .send_channel_response(scope, channel, bytes)
        .await
        .map_err(|_| Stop)
}

/// Content as adjacent frames of at most [`CHUNK_SIZE`] each, `frame`
/// encoding every piece — [`encoded`] of the frame that wraps it. An
/// empty content sends nothing: the frames are the bytes, and no bytes
/// is no frame.
pub(crate) async fn respond_pieces(
    handle: &Handle,
    scope: u32,
    channel: u32,
    content: &[u8],
    frame: impl Fn(&[u8]) -> Option<Vec<u8>>,
) -> Result<(), Stop> {
    for piece in content.chunks(CHUNK_SIZE) {
        let bytes = frame(piece).ok_or(Stop)?;
        respond_bytes(handle, scope, channel, &bytes).await?;
    }
    Ok(())
}

/// The finish: the answer complete.
pub(crate) async fn finish(handle: &Handle, scope: u32, channel: u32) -> Result<(), Stop> {
    handle
        .send_channel_response_finish(scope, channel)
        .await
        .map_err(|_| Stop)
}
