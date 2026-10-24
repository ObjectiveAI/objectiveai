//! The begin scope of a run, whichever family's.

use diverge_sdk::wire::client::handle::SendError;
use diverge_sdk::container_proxy::outside::endpoints::agents::begin::client::execute::{Chunks, ExecuteHandle as AgentsBegin};
use diverge_sdk::container_proxy::outside::client::{Ask, Asks};
use diverge_sdk::container_proxy::outside::endpoints::tools::begin::client::execute::{Events, ExecuteHandle as ToolsBegin};
use diverge_sdk::provider::endpoints::containers::client::answered::{Daemon, Info, Postgres, Secrets};
use diverge_sdk::provider::endpoints::containers::client::{ChannelStream, OpenError, UnaryError};
use diverge_sdk::shared::containers::dependencies::Template;
use diverge_sdk::shared::containers::info::Document;
use diverge_sdk::shared::containers::secrets::{Declaration, Values};

/// The begin scope on a container's proxy connection: the place the
/// proxy's asks ride, and the family's own exchanges.
///
/// One per run, made by [`Runs::begin`](super::family::Runs::begin),
/// held by the [`Run`](super::run::Run) for the container's life. The
/// two families' handles answer the proxy the same way, and ask it
/// the same three things — a database half, the image's `/DIVERGE.md`,
/// a feed of the secrets — which is what the methods here are for;
/// what differs — the queue's exchanges, or the MCP five — the
/// family's own `serve` reaches through the variant.
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

    /// This end's half of a daemon connection the proxy announced,
    /// quoting the proxy's id: a tool container's begin alone has
    /// the channel, and an agent container's answers `None`, since an
    /// agent asks for no daemon connection and its wire has no frame
    /// for one.
    pub(crate) async fn daemon(&self, connection_id: u32) -> Option<Result<ChannelStream<Daemon>, OpenError>> {
        match self {
            Begin::Agents(_) => None,
            Begin::Tools(begin) => Some(begin.daemon(connection_id).await),
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

    /// What the arguments may be, as the image states it: the info
    /// channel on either family's begin, answered with the image's
    /// `/DIVERGE.md` parsed — `None` for an image that ships none —
    /// or the proxy's refusal, which is the file unreadable or not a
    /// document. The proxy reads the file from the container's root;
    /// the program is never asked.
    pub(crate) async fn info(&self) -> Result<Option<Document>, UnaryError<Info>> {
        match self {
            Begin::Agents(begin) => begin.info().await,
            Begin::Tools(begin) => begin.info().await,
        }
    }

    /// The secrets' values, fed to the container: the secrets channel
    /// on either family's begin, answered once the program has
    /// applied them and the proxy's gate is open — or with the
    /// program's refusal, in its own words.
    pub(crate) async fn secrets(&self, values: Values) -> Result<(), UnaryError<Secrets>> {
        match self {
            Begin::Agents(begin) => begin.secrets(values).await,
            Begin::Tools(begin) => begin.secrets(values).await,
        }
    }

    /// The tools family's handle; [`None`] for an agent container.
    pub(crate) fn tools(&self) -> Option<ToolsBegin> {
        match self {
            Begin::Tools(begin) => Some(begin.clone()),
            Begin::Agents(_) => None,
        }
    }
}

/// What a begin hands the machinery: the scope, the asks the proxy
/// will open on it, what the container declared at registration, and
/// what rides the begin's main stream — for an agent container the
/// conversation, for a tool container its activity.
#[derive(Debug)]
pub(crate) struct Begun {
    /// The scope.
    pub begin: Begin,
    /// The channels the proxy opens on it, to relay.
    pub asks: Asks<Ask>,
    /// The agent's chunks, off the begin's main stream, with the
    /// loop's `Active` and `Inactive` among them; [`None`] for a tool
    /// container.
    pub chunks: Option<Chunks>,
    /// The tool container's activity, off the begin's main stream:
    /// `Active` and `Inactive`, and the begin's end; [`None`] for an
    /// agent container, whose chunks end at the same finish.
    pub activity: Option<Events>,
    /// The dependencies an agent container declared at registration,
    /// off its `Begun`: what the caller is asked to deploy, when any.
    /// A tool container's is empty: it declares nothing.
    pub dependencies: Vec<Template>,
    /// The secrets the container takes, by name and mode, off its
    /// `Begun`: what the caller is asked the values of, when any, and
    /// what is fed back before the id goes out.
    pub secrets: Declaration,
}
