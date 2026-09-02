//! The MCP proxy's queue routes, as this container calls them.

use diverge_provider_sdk::agentic_loop_container::{dequeue, enqueue};
use serde::Deserialize;

/// Where the proxy serves beside us: the Container specification's
/// MCP port, on loopback.
const ADDRESS: &str = "http://127.0.0.1:8081";

/// The proxy's answer to an enqueue — its own vocabulary, not the
/// protocol's: `attached` is the fold, not a fate.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Enqueued {
    /// Folded onto a tool response, at its head, the moment that
    /// response went to the agent. The hash is the caller's
    /// correlation key; here it is only the sign that the fold
    /// happened.
    Attached {
        /// The SHA-256 of the folded response's text.
        attached: String,
    },
    /// Withdrawn by a dequeue before any tool response came.
    Dequeued {
        /// Always `true`.
        dequeued: bool,
    },
}

/// Hand the proxy a prompt to fold onto the next tool response.
/// Returns when the fold happens — however long that takes; the
/// proxy holds the request open until then — or when a dequeue
/// withdraws it.
pub async fn enqueue(prompt: String) -> Result<Enqueued, reqwest::Error> {
    reqwest::Client::new()
        .post(format!("{ADDRESS}/enqueue"))
        .json(&enqueue::Request { prompt })
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

/// Withdraw everything the proxy still holds. Every open enqueue
/// answers `dequeued`.
pub async fn dequeue() -> Result<(), reqwest::Error> {
    reqwest::Client::new()
        .post(format!("{ADDRESS}/dequeue"))
        .json(&dequeue::Request {})
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
