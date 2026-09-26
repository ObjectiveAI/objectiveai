//! The container's secrets, from the caller's vault.

use std::sync::Arc;

use bytes::Bytes;

use super::send::{Stop, finish, respond};
use crate::provider::client::Vault;
use crate::wire::client::handle::Handle;
use crate::shared::containers::vault;

/// One frame — the value, that there is none, or the error — then
/// the finish.
pub(crate) async fn get<V: Vault>(handle: &Handle, scope: u32, channel: u32, key: String, vault: Arc<V>) -> Result<(), Stop> {
    let answer = vault.get(&key).await;
    let frame = match &answer {
        Ok(Some(value)) => vault::get::response::Frame::Present(value),
        Ok(None) => vault::get::response::Frame::Missing,
        Err(message) => vault::get::response::Frame::Error(message),
    };
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}

/// Ok, or the error, then the finish.
pub(crate) async fn set<V: Vault>(handle: &Handle, scope: u32, channel: u32, key: String, value: Bytes, vault: Arc<V>) -> Result<(), Stop> {
    ack(handle, scope, channel, vault.set(&key, value).await).await
}

/// Ok, or the error, then the finish.
pub(crate) async fn delete<V: Vault>(handle: &Handle, scope: u32, channel: u32, key: String, vault: Arc<V>) -> Result<(), Stop> {
    ack(handle, scope, channel, vault.delete(&key).await).await
}

/// Ok — once the lock is held, however long that takes — or the
/// error, then the finish.
pub(crate) async fn lock<V: Vault>(handle: &Handle, scope: u32, channel: u32, key: String, ttl: u32, vault: Arc<V>) -> Result<(), Stop> {
    ack(handle, scope, channel, vault.lock(&key, ttl).await).await
}

/// Ok, or the error, then the finish.
pub(crate) async fn unlock<V: Vault>(handle: &Handle, scope: u32, channel: u32, key: String, vault: Arc<V>) -> Result<(), Stop> {
    ack(handle, scope, channel, vault.unlock(&key).await).await
}

/// The one-frame answer every mutation shares.
async fn ack(handle: &Handle, scope: u32, channel: u32, result: Result<(), String>) -> Result<(), Stop> {
    let frame = match &result {
        Ok(()) => vault::response::Frame::Ok,
        Err(message) => vault::response::Frame::Error(message),
    };
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}
