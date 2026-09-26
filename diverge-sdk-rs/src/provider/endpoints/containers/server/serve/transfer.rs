//! One file, from this container into another: a read scope on the
//! one proxy, wired into a write scope on the other.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::family::Family;
use super::super::render;
use super::super::run::Run;
use super::write;
use crate::container_proxy::outside::endpoints::filesystem::read::client::execute as read;
use crate::container_proxy::outside::endpoints::filesystem::write::client::execute as write_scope;
use crate::shared::error::Error;

/// Copy the file at `path` in this run's container into the container
/// under `id`, at `destination`, and answer the caller's channel once:
/// transferred, or the error; then the finish.
///
/// # The rule, enforced here
///
/// The caller must be running, or connected to, both containers. For
/// this run's container that holds by construction: the caller has
/// this scope, so it is the runner or a connector the runner admitted.
/// For the other, the [`Directory`](crate::provider::server::directory::Directory)
/// is asked whether the caller's identity is its runner or among its
/// connectors. A caller that is neither, and an id under which nothing
/// runs, are answered alike, with `{"kind":"denied"}`: saying
/// `missing` for the one would tell a stranger which ids exist.
///
/// # The pipe
///
/// A `filesystem::read` scope is opened on this container's proxy and
/// a `filesystem::write` scope on the other's, and the read's pieces
/// are the write's content, as they arrive. Nothing of the file is
/// held, and nothing of it reaches the caller. The read's errors
/// become the write's: the proxy's own words where it refused, this
/// end's rendering of a connection that broke, which the write
/// abandons — the destination is as it was. The write's answer is the
/// transfer's, made as a write's is. A target run that ends
/// mid-transfer closes its proxy connection, and the write answers
/// that.
pub(crate) async fn transfer<F: Family>(run: Arc<Run>, channel: u32, path: Vec<String>, id: String, destination: Vec<String>) {
    let answer = copy::<F>(&run, path, &id, destination).await;
    run.respond(channel, answer).await;
    run.finish(channel).await;
}

/// The transfer, to its one answer.
async fn copy<F: Family>(run: &Run, path: Vec<String>, id: &str, destination: Vec<String>) -> Option<Vec<u8>> {
    if !run.directory.may(id, &run.identity) {
        return F::transfer_error(&denied());
    }
    let Some(target) = run.directory.lookup(id) else {
        return F::transfer_error(&denied());
    };
    let pieces = match read::execute(&run.proxy, path).await {
        Ok(pieces) => pieces,
        Err(error) => return F::transfer_error(&render::proxy(error)),
    };
    let content = pieces.map(|item| match item {
        Ok(bytes) => Ok(bytes),
        Err(read::ExecuteStreamError::Refused(error)) => Err(error),
        Err(error) => Err(render::proxy(error)),
    });
    match write_scope::execute(&target.proxy, destination, content).await {
        Ok(()) => F::transferred(),
        Err(error) => write::failed(error).and_then(|error| F::transfer_error(&error)),
    }
}

/// The caller is neither running nor connected to the container it
/// named, or nothing runs under the id: one answer for both.
fn denied() -> Error {
    Error(serde_json::json!({ "kind": "denied" }))
}
