//! The daemon, as the courier reaches it: looking an agent up, and
//! notifying one.

use diverge_sdk::container_proxy::inside::Client;
use diverge_sdk::daemon::endpoints::agents::{get, list, message};
use diverge_sdk::daemon::reference;
use diverge_sdk::wire::client::handle::Handle;
use futures_util::StreamExt as _;
use rmcp::model::ContentBlock;
use tokio::sync::Mutex;

use crate::caller::Participant;
use crate::failure::Failure;

/// The one notification, word for word: all the courier ever puts
/// into an agent's conversation.
pub const NOTIFICATION: &str = "<courier>\nYou have a new message\n</courier>";

/// The connection to the daemon, through the proxy, served under the
/// courier's dependency template's grants — `self` and
/// `never_active` over agents: dialled on the first use, kept, and
/// dialled again when an ask finds it gone.
#[derive(Default)]
pub struct Daemon {
    handle: Mutex<Option<Handle>>,
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

    /// The agent `to` names, as the daemon has it: its template, its
    /// index and its name. No such agent is [`Failure::NoAgent`]; one
    /// the courier's account may not see is [`Failure::Forbidden`].
    pub async fn agent(&self, client: &Client, to: &reference::Agent) -> Result<Participant, Failure> {
        let request = get::client::request::Frame { agent: to.clone(), relay: None };
        let handle = self.handle(client).await?;
        let answer = match get::client::execute::execute(&handle, &request).await {
            Ok(answer) => answer,
            Err(error) => {
                self.drop_handle().await;
                return Err(Failure::Daemon(error.to_string()));
            }
        };
        match answer {
            get::server::response::Frame::Found(agent) => Ok(Participant { template: agent.template, index: agent.index as i64, name: agent.name }),
            get::server::response::Frame::NotFound => Err(Failure::NoAgent),
            get::server::response::Frame::Forbidden => Err(Failure::Forbidden),
            get::server::response::Frame::Error(error) => Err(Failure::Daemon(error.0.to_string())),
        }
    }

    /// The agents `filter` lets through that the courier's grants
    /// reach, as a list reports them: the list read to its `listed`
    /// and then cancelled. Forbidden is [`Failure::Forbidden`]; the
    /// daemon's error, a stream that failed, or a list that would not
    /// open is [`Failure::Daemon`].
    pub async fn list(&self, client: &Client, filter: &list::client::request::Filter) -> Result<Vec<list::server::response::Agent>, Failure> {
        let request = list::client::request::Frame { filter: filter.clone(), count: None, relay: None };
        let handle = self.handle(client).await?;
        let (mut stream, cancel) = match list::client::execute::execute(&handle, &request).await {
            Ok(opened) => opened,
            Err(error) => {
                self.drop_handle().await;
                return Err(Failure::Daemon(error.to_string()));
            }
        };
        let mut agents = Vec::new();
        let outcome = loop {
            match stream.next().await {
                Some(Ok(list::server::response::Frame::Added(agent))) => agents.push(agent),
                Some(Ok(list::server::response::Frame::Listed)) => break Ok(agents),
                Some(Ok(list::server::response::Frame::Forbidden)) => break Err(Failure::Forbidden),
                Some(Ok(list::server::response::Frame::Error(error))) => break Err(Failure::Daemon(error.0.to_string())),
                Some(Ok(list::server::response::Frame::Changed(_) | list::server::response::Frame::Removed(_))) => {}
                Some(Err(error)) => break Err(Failure::Daemon(error.to_string())),
                None => break Err(Failure::Daemon("the list ended before it was whole".to_string())),
            }
        };
        let _ = cancel.cancel().await;
        outcome
    }

    /// Send `to` the [`NOTIFICATION`]. Succeeds once the message's
    /// scope is open: the daemon's answer comes only when the agent
    /// takes the message, which may be long after, and a send cannot
    /// wait for it — two agents writing to each other mid-loop would
    /// each wait on the other. That answer is read on a task of its
    /// own, and dropped: the mail is kept whatever it says.
    pub async fn notify(&self, client: &Client, to: &Participant) -> Result<(), Failure> {
        let request = message::client::request::Frame { agent: to.reference(), content: vec![ContentBlock::text(NOTIFICATION)], relay: None };
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
