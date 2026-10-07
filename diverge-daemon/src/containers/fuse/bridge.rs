//! A provider's volume answered through its serve: every ask
//! forwarded under the mount's path, every answer relayed.

use bytes::Bytes;
use diverge_sdk::container_proxy::outside::endpoints::fuse::mount::server::channel_request::{self as ask, Frame as Ask};
use diverge_sdk::provider::client::Listed;
use diverge_sdk::provider::endpoints::volumes::serve::client::execute::ExecuteHandle as ServeHandle;
use diverge_sdk::shared::containers::fuse::ack::{self, Refused};
use diverge_sdk::shared::containers::fuse::stat::Stat;
use diverge_sdk::shared::containers::fuse::{Attrs, list, read, stat};
use tokio::sync::Mutex;

/// The ask's path within the volume: the mount's prefix, then the
/// path as the container gave it.
fn within(prefix: &[String], path: &str) -> String {
    let mut full = prefix.join("/");
    if !path.is_empty() {
        if !full.is_empty() {
            full.push('/');
        }
        full.push_str(path);
    }
    full
}

/// One ask forwarded; the serve gone is the failure.
async fn forward(serve: &Mutex<Option<ServeHandle>>, ask: &Ask<'_>) -> Result<Bytes, String> {
    let held = serve.lock().await;
    let Some(serve) = held.as_ref() else {
        return Err("the volume's serve is over".to_string());
    };
    serve.ask(ask).await.map_err(|error| format!("{error:?}"))
}

/// An ack relayed.
fn acked(answer: Result<Bytes, String>) -> Result<(), Refused> {
    let bytes = answer.map_err(Refused::Error)?;
    match ack::Frame::decode(&bytes) {
        Ok(ack::Frame::Ok) => Ok(()),
        Ok(ack::Frame::ReadOnly) => Err(Refused::ReadOnly),
        Ok(ack::Frame::Error(error)) => Err(Refused::Error(error.to_string())),
        Err(error) => Err(Refused::Error(format!("{error:?}"))),
    }
}

/// What is at the path.
pub async fn stat(serve: &Mutex<Option<ServeHandle>>, prefix: &[String], path: &str) -> Result<Option<Stat>, String> {
    let path = within(prefix, path);
    let bytes = forward(serve, &Ask::Stat(ask::Path { path: &path })).await?;
    match stat::response::Frame::decode(&bytes) {
        Ok(stat::response::Frame::Present(stat)) => Ok(Some(stat)),
        Ok(stat::response::Frame::Missing) => Ok(None),
        Ok(stat::response::Frame::Error(error)) => Err(error.to_string()),
        Err(error) => Err(format!("{error:?}")),
    }
}

/// A piece of the file at the path.
pub async fn read(serve: &Mutex<Option<ServeHandle>>, prefix: &[String], path: &str, offset: u64, length: u32) -> Result<Option<Bytes>, String> {
    let path = within(prefix, path);
    let bytes = forward(serve, &Ask::Read(ask::Read { path: &path, offset, length })).await?;
    match read::response::Frame::decode(&bytes) {
        Ok(read::response::Frame::Present(body)) => Ok(Some(Bytes::copy_from_slice(body))),
        Ok(read::response::Frame::Missing) => Ok(None),
        Ok(read::response::Frame::Error(error)) => Err(error.to_string()),
        Err(error) => Err(format!("{error:?}")),
    }
}

/// A piece written.
pub async fn write(serve: &Mutex<Option<ServeHandle>>, prefix: &[String], path: &str, offset: u64, bytes: &[u8]) -> Result<(), Refused> {
    let path = within(prefix, path);
    acked(forward(serve, &Ask::Write(ask::Write { path: &path, offset, bytes })).await)
}

/// The file made `size` long.
pub async fn truncate(serve: &Mutex<Option<ServeHandle>>, prefix: &[String], path: &str, size: u64) -> Result<(), Refused> {
    let path = within(prefix, path);
    acked(forward(serve, &Ask::Truncate(ask::Truncate { path: &path, size })).await)
}

/// The attributes changed.
pub async fn setattr(serve: &Mutex<Option<ServeHandle>>, prefix: &[String], path: &str, attrs: Attrs) -> Result<(), Refused> {
    let path = within(prefix, path);
    acked(forward(serve, &Ask::Setattr(ask::Setattr { path: &path, attrs })).await)
}

/// The entries of the directory at the path.
pub async fn list(serve: &Mutex<Option<ServeHandle>>, prefix: &[String], path: &str) -> Result<Option<Vec<Listed>>, String> {
    let path = within(prefix, path);
    let bytes = forward(serve, &Ask::List(ask::Path { path: &path })).await?;
    match list::response::Frame::decode(&bytes) {
        Ok(list::response::Frame::Entries(entries)) => Ok(Some(
            entries
                .into_iter()
                .map(|entry| Listed {
                    name: entry.name.to_string(),
                    kind: entry.kind,
                })
                .collect(),
        )),
        Ok(list::response::Frame::Missing) => Ok(None),
        Ok(list::response::Frame::Error(error)) => Err(error.to_string()),
        Err(error) => Err(format!("{error:?}")),
    }
}

/// What is at the path removed.
pub async fn remove(serve: &Mutex<Option<ServeHandle>>, prefix: &[String], path: &str) -> Result<(), Refused> {
    let path = within(prefix, path);
    acked(forward(serve, &Ask::Remove(ask::Path { path: &path })).await)
}

/// What is at `from` moved to `to`.
pub async fn rename(serve: &Mutex<Option<ServeHandle>>, prefix: &[String], from: &str, to: &str) -> Result<(), Refused> {
    let from = within(prefix, from);
    let to = within(prefix, to);
    acked(forward(serve, &Ask::Rename(ask::Rename { from: &from, to: &to })).await)
}

/// A directory made at the path.
pub async fn mkdir(serve: &Mutex<Option<ServeHandle>>, prefix: &[String], path: &str) -> Result<(), Refused> {
    let path = within(prefix, path);
    acked(forward(serve, &Ask::Mkdir(ask::Path { path: &path })).await)
}
