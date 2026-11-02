//! A container opened for a file operation: an agent's run or a
//! tool's, one shape.

use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::shared::filetree::response::Frame;
use futures_util::{Stream, StreamExt as _};

use super::fuse::Mounts;
use super::{AgentRun, Key, ToolRun, Use};
use crate::content::Pieces;

/// A container's tree as it is watched: a snapshot, then every
/// change, until the provider ends it or an error does.
pub type Frames = Pin<Box<dyn Stream<Item = Result<Frame, String>> + Send>>;

/// A command's stream, as a file operation's caller reads it: every
/// frame the container sent, in order, until the command ended, the
/// container stopped, or the stream broke, which is the error last.
pub type ExecFrames = Pin<Box<dyn Stream<Item = Result<diverge_sdk::shared::containers::exec::response::Owned, String>> + Send>>;

/// A held terminal's output, as a file operation's caller reads it:
/// its scrollback, then every frame as it comes, until the shell
/// ended, the container stopped, or the stream broke, which is the
/// error last.
pub type TerminalFrames = Pin<Box<dyn Stream<Item = Result<diverge_sdk::shared::containers::terminal::output::Owned, String>> + Send>>;

/// A container a file operation works on.
#[derive(Clone)]
pub enum Opened {
    /// An agent's run, and the file use this is, which holds the run
    /// up until the last clone is dropped.
    Agent(Arc<AgentRun>, Arc<Use>),
    /// A tool's run, or the connect scope held on another daemon's
    /// tool, whose every file operation answers an error; and, for a
    /// run here, the file use this is, which holds the run up until
    /// the last clone is dropped — a dependency tool's on its agent's
    /// run, with which it stops.
    Tool(Arc<ToolRun>, Option<Arc<Use>>),
}

impl Opened {
    /// The container, by record.
    pub fn key(&self) -> Key {
        match self {
            Opened::Agent(run, _) => Key::Agent(run.id),
            Opened::Tool(run, _) => Key::Tool(run.id),
        }
    }

    /// The mounts the daemon serves into it.
    pub fn mounts(&self) -> &Arc<Mounts> {
        match self {
            Opened::Agent(run, _) => &run.mounts,
            Opened::Tool(run, _) => &run.mounts,
        }
    }

    /// The provider it runs on, or is connected through.
    pub fn provider(&self) -> &Identity {
        match self {
            Opened::Agent(run, _) => &run.provider,
            Opened::Tool(run, _) => &run.provider,
        }
    }

    /// The container's id on its provider, which a transfer into it
    /// names; a connected tool has none here, its container being the
    /// other daemon's.
    pub fn container(&self) -> Option<&str> {
        match self {
            Opened::Agent(run, _) => Some(&run.container),
            Opened::Tool(run, _) => run.container.as_deref(),
        }
    }

    /// The container was used.
    pub fn touch(&self) {
        match self {
            Opened::Agent(run, _) => run.touch(),
            Opened::Tool(run, _) => run.touch(),
        }
    }

    /// The container's tree, watched: the provider's stream.
    pub async fn filetree(&self) -> Result<Frames, String> {
        match self {
            Opened::Agent(run, _) => {
                let stream = run.handle.filetree().await.map_err(|error| error.to_string())?;
                // The channel closing with no finish is the run's scope
                // ending — the container stopped — which ends a watch
                // as the run over, not as a failure.
                let stream = stream.take_while(|frame| std::future::ready(!matches!(frame, Err(diverge_sdk::provider::endpoints::containers::client::ChannelStreamError::Closed))));
                Ok(Box::pin(stream.map(|frame| frame.map_err(|error| error.to_string()))))
            }
            Opened::Tool(run, _) => run.handle.filetree().await,
        }
    }

    /// One file read out of the container.
    pub async fn read(&self, path: Vec<String>) -> Result<Pieces, String> {
        match self {
            Opened::Agent(run, _) => {
                let stream = run.handle.read(path).await.map_err(|error| error.to_string())?;
                Ok(pieces_of(stream))
            }
            Opened::Tool(run, _) => run.handle.read(path).await,
        }
    }

    /// One command run in the container, its stream (see
    /// [`exec`](diverge_sdk::shared::containers::exec)). Dropping the
    /// stream abandons it, which kills the command. The channel closing
    /// with no finish is the container stopped: the stream's end.
    pub async fn exec(&self, command: diverge_sdk::shared::containers::exec::request::Request) -> Result<ExecFrames, String> {
        match self {
            Opened::Agent(run, _) => {
                let stream = run.handle.exec(command).await.map_err(|error| error.to_string())?;
                let stream = stream.take_while(|frame| std::future::ready(!matches!(frame, Err(diverge_sdk::provider::endpoints::containers::client::ChannelStreamError::Closed))));
                Ok(Box::pin(stream.map(|frame| frame.map_err(|error| error.to_string()))))
            }
            Opened::Tool(run, _) => run.handle.exec(command).await,
        }
    }

    /// A terminal started in the container, held by the provider: its
    /// id (see [`session`](diverge_sdk::shared::containers::terminal::session)).
    pub async fn terminal_open(&self, size: diverge_sdk::shared::containers::terminal::open::Open) -> Result<u32, String> {
        match self {
            Opened::Agent(run, _) => run.handle.terminal_open(size).await.map_err(|error| error.to_string()),
            Opened::Tool(run, _) => run.handle.terminal_open(size).await,
        }
    }

    /// A held terminal's output. Dropping the stream detaches; the
    /// terminal goes on. The channel closing with no finish is the
    /// container stopped: the stream's end.
    pub async fn terminal_attach(&self, id: u32) -> Result<TerminalFrames, String> {
        match self {
            Opened::Agent(run, _) => {
                let stream = run.handle.terminal_attach(id).await.map_err(|error| error.to_string())?;
                let stream = stream.take_while(|frame| std::future::ready(!matches!(frame, Err(diverge_sdk::provider::endpoints::containers::client::ChannelStreamError::Closed))));
                Ok(Box::pin(stream.map(|frame| frame.map_err(|error| error.to_string()))))
            }
            Opened::Tool(run, _) => run.handle.terminal_attach(id).await,
        }
    }

    /// Bytes written to a held terminal's stdin.
    pub async fn terminal_stdin(&self, id: u32, bytes: Vec<u8>) -> Result<(), String> {
        match self {
            Opened::Agent(run, _) => run.handle.terminal_stdin(id, bytes).await.map_err(|error| error.to_string()),
            Opened::Tool(run, _) => run.handle.terminal_stdin(id, bytes).await,
        }
    }

    /// A held terminal's size set.
    pub async fn terminal_resize(&self, id: u32, size: diverge_sdk::shared::containers::terminal::open::Open) -> Result<(), String> {
        match self {
            Opened::Agent(run, _) => run.handle.terminal_resize(id, size.cols, size.rows).await.map_err(|error| error.to_string()),
            Opened::Tool(run, _) => run.handle.terminal_resize(id, size).await,
        }
    }

    /// A held terminal's shell killed, and the terminal forgotten.
    pub async fn terminal_close(&self, id: u32) -> Result<(), String> {
        match self {
            Opened::Agent(run, _) => run.handle.terminal_close(id).await.map_err(|error| error.to_string()),
            Opened::Tool(run, _) => run.handle.terminal_close(id).await,
        }
    }

    /// One file written into the container, replaced whole.
    pub async fn write(&self, path: Vec<String>, content: Pieces) -> Result<(), String> {
        let content = content.map(|piece| piece.map_err(Source));
        match self {
            Opened::Agent(run, _) => run.handle.write(path, content).await.map_err(|error| error.to_string()),
            Opened::Tool(run, _) => run.handle.write(path, content).await,
        }
    }

    /// One file copied into the container under `id` on the same
    /// provider, the bytes staying there.
    pub async fn transfer(&self, path: Vec<String>, id: String, destination: Vec<String>) -> Result<(), String> {
        match self {
            Opened::Agent(run, _) => run.handle.transfer(path, id, destination).await.map_err(|error| error.to_string()),
            Opened::Tool(run, _) => run.handle.transfer(path, id, destination).await,
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
