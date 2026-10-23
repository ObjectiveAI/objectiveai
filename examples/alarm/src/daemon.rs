//! The daemon, as the alarm tool reaches it: messaging an agent.

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
    handle: Mutex<Option<Handle>>,
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

    /// Forget the held connection, so the next ask dials again.
    async fn drop_handle(&self) {
        *self.handle.lock().await = None;
    }

    /// Send `to` the alarm's [`message()`] with `note`.
    ///
    /// The daemon's answer to a message is the agent's turn delivered,
    /// which can be long; the scheduler does not wait for it, or one
    /// slow agent would hold every other alarm back. That answer is
    /// read on a task of its own, and dropped.
    pub async fn wake(&self, client: &Client, to: &Participant, note: &str) -> Result<(), Failure> {
        let request = message::client::request::Frame { agent: to.reference(), content: vec![ContentBlock::text(message(note))], relay: None };
        let handle = self.handle(client).await?;
        let pending = match message::client::execute::execute(&handle, &request).await {
            Ok(pending) => pending,
            Err(error) => {
                self.drop_handle().await;
                return Err(Failure::Daemon(error.to_string()));
            }
        };
        tokio::spawn(async move {
            let _ = pending.wait().await;
        });
        Ok(())
    }
}
