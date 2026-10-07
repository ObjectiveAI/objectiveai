//! A container opened for a file operation: an agent's run or a
//! tool's, one shape.

use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::shared::filetree::response::Frame;
use futures_util::{Stream, StreamExt as _};

use super::fuse::Mounts;
use super::{AgentRun, Key, ToolRun};
use crate::content::Pieces;

/// A container's tree as it is watched: a snapshot, then every
/// change, until the provider ends it or an error does.
pub type Frames = Pin<Box<dyn Stream<Item = Result<Frame, String>> + Send>>;

/// A container a file operation works on.
#[derive(Clone)]
pub enum Opened {
    /// An agent's run.
    Agent(Arc<AgentRun>),
    /// A tool's run, or the connect scope joined to it.
    Tool(Arc<ToolRun>),
}

impl Opened {
    /// The container, by record.
    pub fn key(&self) -> Key {
        match self {
            Opened::Agent(run) => Key::Agent(run.id),
            Opened::Tool(run) => Key::Tool(run.id),
        }
    }

    /// The mounts the daemon serves into it.
    pub fn mounts(&self) -> &Arc<Mounts> {
        match self {
            Opened::Agent(run) => &run.mounts,
            Opened::Tool(run) => &run.mounts,
        }
    }

    /// The provider it runs on, or is joined through.
    pub fn provider(&self) -> &Identity {
        match self {
            Opened::Agent(run) => &run.provider,
            Opened::Tool(run) => &run.provider,
        }
    }

    /// The container's id on its provider, which a transfer into it
    /// names; a connected tool's id is its runner's.
    pub fn container(&self) -> Option<&str> {
        match self {
            Opened::Agent(run) => Some(&run.container),
            Opened::Tool(run) => run.container.as_deref(),
        }
    }

    /// The container was used.
    pub fn touch(&self) {
        match self {
            Opened::Agent(run) => run.touch(),
            Opened::Tool(run) => run.touch(),
        }
    }

    /// The container's tree, watched: the provider's stream.
    pub async fn filetree(&self) -> Result<Frames, String> {
        match self {
            Opened::Agent(run) => {
                let stream = run.handle.filetree().await.map_err(|error| error.to_string())?;
                Ok(Box::pin(stream.map(|frame| frame.map_err(|error| error.to_string()))))
            }
            Opened::Tool(run) => run.handle.filetree().await,
        }
    }

    /// One file read out of the container.
    pub async fn read(&self, path: Vec<String>) -> Result<Pieces, String> {
        match self {
            Opened::Agent(run) => {
                let stream = run.handle.read(path).await.map_err(|error| error.to_string())?;
                Ok(pieces_of(stream))
            }
            Opened::Tool(run) => run.handle.read(path).await,
        }
    }

    /// One file written into the container, replaced whole.
    pub async fn write(&self, path: Vec<String>, content: Pieces) -> Result<(), String> {
        let content = content.map(|piece| piece.map_err(Source));
        match self {
            Opened::Agent(run) => run.handle.write(path, content).await.map_err(|error| error.to_string()),
            Opened::Tool(run) => run.handle.write(path, content).await,
        }
    }

    /// One file copied into the container under `id` on the same
    /// provider, the bytes staying there.
    pub async fn transfer(&self, path: Vec<String>, id: String, destination: Vec<String>) -> Result<(), String> {
        match self {
            Opened::Agent(run) => run.handle.transfer(path, id, destination).await.map_err(|error| error.to_string()),
            Opened::Tool(run) => run.handle.transfer(path, id, destination).await,
        }
    }
}

/// Why a source's content stopped, as a write's content error.
#[derive(Debug)]
pub struct Source(pub String);

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A read stream's items as pieces.
pub fn pieces_of(stream: impl Stream<Item = Result<Bytes, impl std::fmt::Display>> + Send + 'static) -> Pieces {
    Box::pin(stream.map(|item| item.map_err(|error| error.to_string())))
}
