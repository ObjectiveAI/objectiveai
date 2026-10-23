//! The six commands: the four sends, and the link's two.

use bytes::Bytes;
use tauri::State;
use tauri::ipc::{Channel, Response};
use tokio::sync::mpsc::UnboundedReceiver;

use super::TauriProxy;
use crate::wire::proxy::{Refusal, WireStatus, forward};

/// Open a scope with `payload` — a whole request payload, tag and all
/// — and answer its number. Every frame the scope receives is
/// forwarded on `responses`, and every request the daemon opens in it
/// on `requests`, each one whole, header included; each channel ends
/// with one empty message when its receiver closes.
#[tauri::command]
pub async fn wire_request(proxy: State<'_, TauriProxy>, payload: Vec<u8>, responses: Channel<Response>, requests: Channel<Response>) -> Result<u32, Refusal> {
    let link = proxy.link();
    let handle = link.handle().await?;
    let scope = link.sent(handle.send_request(&payload).await).await?;
    pump(scope.response_receiver, responses);
    pump(scope.request_receiver, requests);
    Ok(scope.scope)
}

/// Open a channel in `scope` with `payload` and answer its number.
/// Every frame the channel receives is forwarded on `responses`,
/// whole, and the channel ends with one empty message when its
/// receiver closes.
#[tauri::command]
pub async fn wire_channel_request(proxy: State<'_, TauriProxy>, scope: u32, payload: Vec<u8>, responses: Channel<Response>) -> Result<u32, Refusal> {
    let link = proxy.link();
    let handle = link.handle().await?;
    let channel = link.sent(handle.send_channel_request(scope, &payload).await).await?;
    pump(channel.response_receiver, responses);
    Ok(channel.channel)
}

/// Answer, with `payload`, on a channel the daemon opened in `scope`.
#[tauri::command]
pub async fn wire_channel_response(proxy: State<'_, TauriProxy>, scope: u32, channel: u32, payload: Vec<u8>) -> Result<(), Refusal> {
    let link = proxy.link();
    let handle = link.handle().await?;
    link.sent(handle.send_channel_response(scope, channel, &payload).await).await
}

/// End an answer on a channel the daemon opened in `scope`.
#[tauri::command]
pub async fn wire_channel_response_finish(proxy: State<'_, TauriProxy>, scope: u32, channel: u32) -> Result<(), Refusal> {
    let link = proxy.link();
    let handle = link.handle().await?;
    link.sent(handle.send_channel_response_finish(scope, channel).await).await
}

/// How the link stands, dialling first when nothing has been dialled
/// since the link was made or reset. A link that is down stays down;
/// this never redials it. Never refused.
#[tauri::command]
pub async fn wire_status(proxy: State<'_, TauriProxy>) -> Result<WireStatus, Refusal> {
    let link = proxy.link();
    let _ = link.handle().await;
    Ok(link.status())
}

/// Put the link back to not yet dialled, so the next use dials. Never
/// refused.
#[tauri::command]
pub async fn wire_reset(proxy: State<'_, TauriProxy>) -> Result<(), Refusal> {
    proxy.link().reset().await;
    Ok(())
}

/// Forward every frame `receiver` yields on `channel`, whole, then the
/// end — one empty message — when the receiver closes:
/// [`forward()`], with the page's channel as its delivery. A delivery
/// that fails is the page gone, which ends the forwarding and drops
/// the receiver. The bytes cross as a raw response: a channel of plain
/// bytes would cross as a JSON list of numbers.
fn pump(receiver: UnboundedReceiver<Bytes>, channel: Channel<Response>) {
    tauri::async_runtime::spawn(forward(receiver, move |frame| channel.send(Response::new(frame.map(|bytes| bytes.to_vec()).unwrap_or_default())).is_ok()));
}
