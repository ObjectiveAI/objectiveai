//! The commands the page invokes: the SDK's four, and the link's two.

use bytes::Bytes;
use tauri::State;
use tauri::ipc::{Channel, Response};
use tokio::sync::mpsc::UnboundedReceiver;

use super::{Wire, WireStatus};

/// Open a scope with `payload` — a whole request payload, tag and all
/// — and answer its number. Every frame the scope receives is
/// forwarded on `responses`, and every request the daemon opens in it
/// on `requests`, each one whole, header included; the scope's finish
/// is forwarded too, and closes both.
#[tauri::command]
pub async fn wire_request(wire: State<'_, Wire>, payload: Vec<u8>, responses: Channel<Response>, requests: Channel<Response>) -> Result<u32, String> {
    let handle = wire.handle().await?;
    let scope = wire.sent(handle.send_request(&payload).await).await?;
    pump(scope.response_receiver, responses);
    pump(scope.request_receiver, requests);
    Ok(scope.scope)
}

/// Open a channel in `scope` with `payload` and answer its number.
/// Every frame the channel receives is forwarded on `responses`,
/// whole, the channel's finish last.
#[tauri::command]
pub async fn wire_channel_request(wire: State<'_, Wire>, scope: u32, payload: Vec<u8>, responses: Channel<Response>) -> Result<u32, String> {
    let handle = wire.handle().await?;
    let channel = wire.sent(handle.send_channel_request(scope, &payload).await).await?;
    pump(channel.response_receiver, responses);
    Ok(channel.channel)
}

/// Answer, with `payload`, on a channel the daemon opened in `scope`.
#[tauri::command]
pub async fn wire_channel_response(wire: State<'_, Wire>, scope: u32, channel: u32, payload: Vec<u8>) -> Result<(), String> {
    let handle = wire.handle().await?;
    wire.sent(handle.send_channel_response(scope, channel, &payload).await).await
}

/// End an answer on a channel the daemon opened in `scope`.
#[tauri::command]
pub async fn wire_channel_response_finish(wire: State<'_, Wire>, scope: u32, channel: u32) -> Result<(), String> {
    let handle = wire.handle().await?;
    wire.sent(handle.send_channel_response_finish(scope, channel).await).await
}

/// How the link stands, dialling first when nothing is up: the page's
/// first call, and its retry after a reset.
#[tauri::command]
pub async fn wire_status(wire: State<'_, Wire>) -> Result<WireStatus, String> {
    let _ = wire.handle().await;
    Ok(wire.status())
}

/// Drop the link, so the next use dials again.
#[tauri::command]
pub async fn wire_reset(wire: State<'_, Wire>) -> Result<(), String> {
    wire.reset().await;
    Ok(())
}

/// Forward every frame `receiver` yields on `channel`, whole, until
/// the receiver closes — the scope or the channel is over — or the
/// page stops listening, which drops the receiver and with it the
/// queue nobody reads. The bytes cross as a raw response: a channel
/// of plain bytes would cross as a JSON list of numbers.
fn pump(mut receiver: UnboundedReceiver<Bytes>, channel: Channel<Response>) {
    tauri::async_runtime::spawn(async move {
        while let Some(frame) = receiver.recv().await {
            if channel.send(Response::new(frame.to_vec())).is_err() {
                return;
            }
        }
    });
}
