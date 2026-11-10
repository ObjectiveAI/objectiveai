//! The daemon, as the alarm tool reaches it: messaging an agent.

use std::sync::Arc;

use diverge_sdk::container_proxy::inside::Client;
use diverge_sdk::daemon::endpoints::agents::message;
use diverge_sdk::wire::client::handle::Handle;
use rmcp::model::ContentBlock;
use tokio::sync::Mutex;

use crate::caller::Participant;
use crate::failure::Failure;

/// The connection to the daemon, through the proxy, served under the
/// alarm's dependency template's grants — `message` over the agents
/// attached to it: dialled on the first use, kept, and dialled again
/// when an ask finds it gone.
#[derive(Default)]
pub struct Daemon {
    handle: Arc<Mutex<Option<Handle>>>,
}

/// The message an alarm sends, word for word: `note` between the
/// tags, each on its own line.
pub fn message(note: &str) -> String {
    format!("<alarm>\n{note}\n</alarm>")
}

impl Daemon {
    /// The handle: the one held, or a new one dialled.
    async fn handle(&self, client: &Client) -> Result<Handle, Failure> {
        let mut held = self.handle.lock().await;
        if let Some(handle) = held.as_ref() {
            return Ok(handle.clone());
        }
        let handle = client.daemon().await.map_err(|error| Failure::Daemon(error.to_string()))?;
        *held = Some(handle.clone());
        Ok(handle)
    }

    /// Send `to` the alarm's [`message()`] with `note`.
    ///
    /// The daemon's answer to a message comes only when the agent's
    /// program accepts it, which may be a whole start away; the
    /// scheduler does not wait for it, or one slow agent would hold
    /// every other alarm back. The message is sent and its answer
    /// read on a task of its own, and dropped.
    pub async fn wake(&self, client: &Client, to: &Participant, note: &str) -> Result<(), Failure> {
        let request = message::client::request::Frame { agent: to.reference(), content: vec![ContentBlock::text(message(note))], relay: None };
        let handle = self.handle(client).await?;
        let held = self.handle.clone();
        tokio::spawn(async move {
            // A send that failed forgets the held connection, so the next ask dials again.
            if message::client::execute::execute(&handle, &request).await.is_err() {
                *held.lock().await = None;
            }
        });
        Ok(())
    }
}
