//! The daemon, as the chat reaches it: the attached agents, their
//! images, and the notification.

use std::collections::HashMap;
use std::sync::Arc;

use diverge_sdk::container_proxy::inside::Client;
use diverge_sdk::daemon::endpoints::agents::templates::get as template_get;
use diverge_sdk::daemon::endpoints::agents::{list, message};
use diverge_sdk::daemon::endpoints::images::info;
use diverge_sdk::shared::containers::info::Document;
use diverge_sdk::shared::containers::request::Image;
use diverge_sdk::wire::client::handle::Handle;
use futures_util::StreamExt as _;
use rmcp::model::ContentBlock;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::caller::Participant;
use crate::failure::Failure;

/// The one notification, word for word: all the chat ever puts into an
/// agent's conversation.
pub const NOTIFICATION: &str = "<chat>\nYou have a new message\n</chat>";

/// An attached agent, as the page shows it: who it is, and its image —
/// the image's name and its `/DIVERGE.md`, where the image ships one.
#[derive(Debug, Clone, Serialize)]
pub struct Shown {
    /// The agent's template, by id.
    pub template: String,
    /// The agent's index among those made from the template.
    pub index: i64,
    /// The agent's name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The image's digest.
    pub digest: String,
    /// The image's `/DIVERGE.md`, or `None` for an image that ships
    /// none, or one that could not be read.
    pub document: Option<Document>,
}

/// The connection to the daemon, through the proxy, served under the
/// chat's grants: dialled on first use, kept, and dialled again when an
/// ask finds it gone. Each image's document is asked once and kept.
#[derive(Default)]
pub struct Daemon {
    handle: Arc<Mutex<Option<Handle>>>,
    documents: Mutex<HashMap<String, Option<Document>>>,
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

    /// Every agent attached to the chat, with its image: the list read
    /// to its `listed` and then cancelled, then each agent's template
    /// and its image's document.
    pub async fn attached(&self, client: &Client) -> Result<Vec<Shown>, Failure> {
        let handle = self.handle(client).await?;
        let request = list::client::request::Frame { filter: Default::default(), count: None, relay: None };
        let (mut stream, cancel) = match list::client::execute::execute(&handle, &request).await {
            Ok(opened) => opened,
            Err(error) => {
                self.drop_handle().await;
                return Err(Failure::Daemon(error.to_string()));
            }
        };
        let mut agents = Vec::new();
        let listed = loop {
            match stream.next().await {
                Some(Ok(list::server::response::Frame::Added(agent))) => agents.push(agent),
                Some(Ok(list::server::response::Frame::Listed)) => break Ok(()),
                Some(Ok(list::server::response::Frame::Forbidden)) => break Err(Failure::Daemon("the chat may not list agents".to_string())),
                Some(Ok(list::server::response::Frame::Error(error))) => break Err(Failure::Daemon(error.0.to_string())),
                Some(Ok(list::server::response::Frame::Changed(_) | list::server::response::Frame::Removed(_))) => {}
                Some(Err(error)) => break Err(Failure::Daemon(error.to_string())),
                None => break Err(Failure::Daemon("the list ended before it was whole".to_string())),
            }
        };
        let _ = cancel.cancel().await;
        listed?;
        let mut shown = Vec::with_capacity(agents.len());
        for agent in agents {
            let image = self.image(&handle, &agent.template).await?;
            let document = self.document(&handle, &image).await;
            shown.push(Shown { template: agent.template, index: agent.index as i64, name: agent.name, digest: image.digest, document });
        }
        shown.sort_by(|a, b| (&a.template, a.index).cmp(&(&b.template, b.index)));
        Ok(shown)
    }

    /// The image an agent template names.
    async fn image(&self, handle: &Handle, template: &str) -> Result<Image, Failure> {
        let request = template_get::client::request::Frame { id: template.to_string(), relay: None };
        let answer = template_get::client::execute::execute(handle, &request).await.map_err(|error| Failure::Daemon(error.to_string()))?;
        match answer {
            template_get::server::response::Frame::Found(listed) => {
                // The template's image and a container's are one JSON
                // shape: a digest and its references.
                let value = serde_json::to_value(&listed.template.image).map_err(|error| Failure::Daemon(error.to_string()))?;
                serde_json::from_value(value).map_err(|error| Failure::Daemon(error.to_string()))
            }
            template_get::server::response::Frame::NotFound => Err(Failure::Daemon(format!("no agent template {template}"))),
            template_get::server::response::Frame::Forbidden => Err(Failure::Daemon("the chat may not read agent templates".to_string())),
            template_get::server::response::Frame::Error(error) => Err(Failure::Daemon(error.0.to_string())),
        }
    }

    /// An image's `/DIVERGE.md`, asked once per digest and kept; `None`
    /// for an image that ships none or could not be read, which is kept
    /// only when the image ships none.
    async fn document(&self, handle: &Handle, image: &Image) -> Option<Document> {
        if let Some(known) = self.documents.lock().await.get(&image.digest) {
            return known.clone();
        }
        let request = info::client::request::Frame { image: image.clone(), ignore: Vec::new(), relay: None };
        let answer = info::client::execute::execute(handle, &request).await.ok()?;
        let (document, kept) = match answer {
            info::server::response::Frame::Info(document) => (Some(document), true),
            info::server::response::Frame::Missing => (None, true),
            _ => (None, false),
        };
        if kept {
            self.documents.lock().await.insert(image.digest.clone(), document.clone());
        }
        document
    }

    /// Send `to` the [`NOTIFICATION`]. Succeeds once the connection
    /// to the daemon is in hand: the daemon's answer comes only when
    /// the agent's program accepts the message, which may be a whole
    /// start away, so the message is sent and its answer read on a
    /// task of its own, and dropped.
    pub async fn notify(&self, client: &Client, to: &Participant) -> Result<(), Failure> {
        let request = message::client::request::Frame { agent: to.reference(), content: vec![ContentBlock::text(NOTIFICATION)], relay: None };
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
