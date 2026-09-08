//! The `openrouter` agent, as a container.
//!
//! The program an agent container's server runs for an agent whose
//! `upstream` is `openrouter`. It has one job: attach to the proxy
//! beside it, wait for the request the server hands over, run the
//! loop, stream the chunks back, and exit — the container's life is
//! this program's. Its tool calls go through the proxy's MCP server;
//! its key comes from the vault the caller holds; the history it
//! resumes from, and leaves behind, is one row in the caller's
//! database, reached through the proxy's loopback pgwire.
//!
//! # What is an error, and what is not
//!
//! Anything that fails before the loop has said a single thing is a
//! real error — the wire's `Error` frame, with a JSON reason, then
//! the close: an agent value that is not an openrouter agent, a vault
//! with no `OPENROUTER_API_KEY`, a database that will not answer, a
//! history that will not open, the first fetch. A failure AFTER
//! output is a fatal `notification` chunk, the loop's own last word,
//! then the close. The first item decides which.

mod continuation;
mod fetch;
mod r#loop;
// The wire modules carry OpenRouter's COMPLETE shapes, which is more
// than this container constructs or reads — a role never built, a
// field parsed and never consumed. The derives used to count as use;
// now that each module keeps only its own direction's derive, the
// completeness reads as dead code, and is not.
#[allow(dead_code)]
mod request;
#[allow(dead_code)]
mod response;
mod serde_util;
mod stream_once;

use diverge_container_proxy_sdk::{Client, RunLoopHandle};
use diverge_provider_sdk::endpoints::containers::agents::agent::openrouter;
use diverge_provider_sdk::shared::containers::run_loop::response::{
    AgenticLoopChunk, NotificationChunk,
};
use diverge_provider_sdk::shared::error::Error as WireError;
use futures_util::StreamExt as _;

use crate::continuation::Continuation;
use crate::r#loop::Item;

/// The vault key the upstream's credential is kept under.
const API_KEY: &str = "OPENROUTER_API_KEY";

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("the runtime could not be built");
    runtime.block_on(run());
}

/// The run: attach, receive, prepare, loop, relay, exit.
///
/// A proxy that cannot be attached to is a container nothing can
/// serve, and there is nobody to tell: this returns, and the
/// container ends.
async fn run() {
    let client = Client::new();
    let Ok((request, handle)) = client.run_loop().await else {
        return;
    };

    let agent: openrouter::Agent = match serde_json::from_value(request.agent) {
        Ok(agent) => agent,
        Err(error) => {
            fail(
                handle,
                serde_json::json!({
                    "kind": "agent",
                    "error": error.to_string(),
                }),
            )
            .await;
            return;
        }
    };

    let api_key = match client.vault_get(API_KEY).await {
        Ok(Some(bytes)) => match String::from_utf8(bytes.to_vec()) {
            Ok(api_key) => api_key,
            Err(_) => {
                fail(
                    handle,
                    serde_json::json!({
                        "kind": "api_key",
                        "error": format!("{API_KEY} is not UTF-8"),
                    }),
                )
                .await;
                return;
            }
        },
        Ok(None) => {
            fail(
                handle,
                serde_json::json!({
                    "kind": "api_key",
                    "error": format!("the vault holds no {API_KEY}"),
                }),
            )
            .await;
            return;
        }
        Err(error) => {
            fail(
                handle,
                serde_json::json!({
                    "kind": "api_key",
                    "error": error.to_string(),
                }),
            )
            .await;
            return;
        }
    };

    let pool = match sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&client.postgres_url())
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            fail(
                handle,
                serde_json::json!({
                    "kind": "continuation",
                    "error": error.to_string(),
                }),
            )
            .await;
            return;
        }
    };
    let continuation = match Continuation::load(&pool).await {
        Ok(continuation) => continuation,
        Err(error) => {
            fail(handle, error.message()).await;
            return;
        }
    };

    let mut items = match r#loop::r#loop(
        &client,
        &api_key,
        agent,
        continuation,
        request.prompt,
    )
    .await
    {
        Ok(items) => items,
        Err(error) => {
            fail(handle, error.message()).await;
            return;
        }
    };

    // The first item decides: an error before any chunk is a real
    // error, after one it is the loop's own fatal notification.
    let mut handle = handle;
    let mut spoke = false;
    while let Some(item) = items.next().await {
        match item {
            Ok(Item::Chunk(chunk)) => {
                spoke = true;
                if handle.send(chunk).await.is_err() {
                    return;
                }
            }
            Ok(Item::Rest(history)) => {
                if let Err(error) = history.save(&pool).await {
                    finish_failed(handle, error.message()).await;
                    return;
                }
            }
            Err(error) => {
                if spoke {
                    finish_failed(handle, error.message()).await;
                } else {
                    fail(handle, error.message()).await;
                }
                return;
            }
        }
    }
    let _ = handle.finish().await;
}

/// A notification chunk, its fatality the caller's verdict.
fn notification(message: serde_json::Value, is_fatal: bool) -> AgenticLoopChunk {
    AgenticLoopChunk::Notification(NotificationChunk {
        r#type: Default::default(),
        is_fatal,
        message,
        meta: None,
    })
}

/// The real error: nothing ran, and this is why. Sent, then the
/// close.
async fn fail(handle: RunLoopHandle, message: serde_json::Value) {
    let _ = handle.error(WireError(message)).await;
}

/// The loop's last word: a fatal notification, then the clean close —
/// the stream had already begun, and what it said stands.
async fn finish_failed(mut handle: RunLoopHandle, message: serde_json::Value) {
    if handle.send(notification(message, true)).await.is_ok() {
        let _ = handle.finish().await;
    }
}
