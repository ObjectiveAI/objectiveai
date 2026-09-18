//! The begin scope of a run, whichever family's.

use serde_json::Value;

use crate::client::handle::SendError;
use crate::container_proxy_endpoints::agents::begin::client::execute::{Chunks, ExecuteHandle as AgentsBegin};
use crate::container_proxy_endpoints::client::{Ask, Asks};
use crate::container_proxy_endpoints::tools::begin::client::execute::{ExecuteHandle as ToolsBegin, Finish};
use crate::endpoints::containers::client::answered::{Postgres, Schema};
use crate::endpoints::containers::client::{ChannelStream, OpenError, UnaryError};

/// The begin scope on a container's proxy connection: the place the
/// proxy's asks ride, and the family's own exchanges.
///
/// One per run, made by [`Runs::begin`](super::family::Runs::begin),
/// held by the [`Run`](super::run::Run) for the container's life. The
/// two families' handles answer the proxy the same way, and ask it
/// the same two things — a database half, the arguments' schema —
/// which is what the methods here are for; what differs — the
/// queue's exchanges, or the MCP five — the family's own `serve`
/// reaches through the variant.
#[derive(Debug)]
pub(crate) enum Begin {
    /// An agent container's.
    Agents(AgentsBegin),
    /// A tool container's.
    Tools(ToolsBegin),
}

impl Begin {
    /// One answer on a channel the proxy opened, by the proxy's
    /// number.
    pub(crate) async fn respond(&self, channel: u32, payload: &[u8]) -> Result<(), SendError> {
        match self {
            Begin::Agents(begin) => begin.respond(channel, payload).await,
            Begin::Tools(begin) => begin.respond(channel, payload).await,
        }
    }

    /// The end of the answers on a channel the proxy opened.
    pub(crate) async fn finish(&self, channel: u32) -> Result<(), SendError> {
        match self {
            Begin::Agents(begin) => begin.finish(channel).await,
            Begin::Tools(begin) => begin.finish(channel).await,
        }
    }

    /// This end's half of a database connection the proxy announced,
    /// quoting the proxy's id.
    pub(crate) async fn postgres(&self, connection_id: u32) -> Result<ChannelStream<Postgres>, OpenError> {
        match self {
            Begin::Agents(begin) => begin.postgres(connection_id).await,
            Begin::Tools(begin) => begin.postgres(connection_id).await,
        }
    }

    /// What the arguments may be, as the container's server states
    /// it: the schema channel on either family's begin.
    pub(crate) async fn schema(&self) -> Result<Value, UnaryError<Schema>> {
        match self {
            Begin::Agents(begin) => begin.schema().await,
            Begin::Tools(begin) => begin.schema().await,
        }
    }

    /// The tools family's handle, which a connector's exchanges
    /// ride; [`None`] for an agent container.
    pub(crate) fn tools(&self) -> Option<ToolsBegin> {
        match self {
            Begin::Tools(begin) => Some(begin.clone()),
            Begin::Agents(_) => None,
        }
    }
}

/// What a begin hands the machinery: the scope, the asks the proxy
/// will open on it, and — for an agent container — the conversation.
#[derive(Debug)]
pub(crate) struct Begun {
    /// The scope.
    pub begin: Begin,
    /// The channels the proxy opens on it, to relay.
    pub asks: Asks<Ask>,
    /// The agent's chunks, off the begin's main stream; [`None`] for
    /// a tool container, whose main stream carries nothing more.
    pub chunks: Option<Chunks>,
    /// The begin's end, for a tool container, whose main stream is
    /// read for nothing else; [`None`] for an agent container, whose
    /// chunks end at the same finish.
    pub finish: Option<Finish>,
}
