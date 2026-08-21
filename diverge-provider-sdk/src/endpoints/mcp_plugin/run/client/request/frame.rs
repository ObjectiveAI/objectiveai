//! What a client's request frame carries for an MCP plugin.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::Identity;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::container::request::Image;

/// Ask a provider to run an MCP plugin.
///
/// The same bargain a
/// [`laboratory run`](crate::endpoints::laboratories::run::client::request::Frame)
/// strikes — a caller chooses the image and the resources, a provider
/// chooses the name, the labels and the entrypoint — with the
/// injection removed and the configuration added.
///
/// # Three ports, and they are all the container's
///
/// [`mcp_port`](Self::mcp_port),
/// [`postgres_port`](Self::postgres_port) and
/// [`command_port`](Self::command_port) are the three sockets a
/// provider opens INTO this container. They are ports inside it, not
/// host ones — what a provider reaches them as is its own business and
/// no caller will see it.
///
/// The caller states them because only the image's author knows what
/// the image binds. A
/// [`laboratory`](crate::endpoints::laboratories::run::client::request::Frame)
/// has none of these: a provider puts the server in a laboratory
/// itself and therefore picks its own numbers. Here everything arrived
/// with the image, already bound to something.
///
/// # A wrong one is not detectable from here
///
/// The container starts fine and nothing answers. Which is true of all
/// three and shows up differently in each: a wrong
/// [`mcp_port`](Self::mcp_port) is an exchange that finishes without a
/// head, and a wrong
/// [`postgres_port`](Self::postgres_port) or
/// [`command_port`](Self::command_port) is a conduit that never
/// carries anything, because the provider connected to nothing.
///
/// # What is missing, and why it is missing rather than ignored
///
/// A laboratory takes `mounts` and an `initial_cwd`. Neither appears
/// here.
///
/// A plugin serves tools; it does not work on a filesystem, and
/// mounting a caller's directories into a container whose whole
/// purpose is to answer tool calls would hand it access it has no
/// reason to want. And a working directory has nothing to apply to:
/// the image's own `WORKDIR` governs its entrypoint, and there is no
/// second process placed alongside it to put anywhere else.
///
/// Stating them and having a provider drop them would be worse than
/// not offering them. A field that is accepted and ignored is a field
/// callers will believe in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The image, and who supplies it.
    ///
    /// See [`Image`]. Read exactly as a laboratory's is — the same
    /// three answers meaning the same three things, since nothing
    /// about supplying an image changes because the container built
    /// from it will serve tool calls.
    pub image: Image,
    /// How much memory the container may have, in BYTES.
    ///
    /// A ceiling, not a hint — a plugin that exceeds what it is
    /// allowed is killed by the kernel rather than told. Which matters
    /// more here than for a laboratory: a laboratory's death is
    /// visible to the agent working in it, while a plugin's shows up
    /// as tool calls that stop being answered.
    pub memory: u64,
    /// How much the plugin may WRITE, in BYTES.
    ///
    /// Its own filesystem only — what it adds to or changes over the
    /// image it came from. The image's layers are read-only and are
    /// not counted, so a plugin starts at nothing however large the
    /// image is.
    ///
    /// And its own filesystem is all it has. A plugin takes no mounts,
    /// so unlike a
    /// [`laboratory's`](crate::endpoints::laboratories::run::client::request::Frame::disk)
    /// there is no second kind of storage for this number to be
    /// distinguished from.
    pub disk: u64,
    /// The environment, name to value.
    ///
    /// For what the IMAGE expects — credentials, endpoints, whatever
    /// its author documented. Not for
    /// [`arguments`](Self::arguments) or [`identity`](Self::identity),
    /// which a provider delivers by its own reserved names and will
    /// overwrite anything here that collides.
    ///
    /// Ordered, so the same environment always serializes identically,
    /// and a map rather than `KEY=VALUE` strings so one name cannot
    /// appear twice with values that contradict each other.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub environment: IndexMap<String, String>,
    /// Where the plugin's MCP server listens, inside the container.
    ///
    /// A provider connects to it and relays a caller's
    /// [`Mcp`](crate::endpoints::mcp_plugin::run::client::channel_request::Frame::Mcp)
    /// exchanges in. It is the only one of the three where the
    /// connection and the requests run the same way round.
    pub mcp_port: u16,
    /// Where the plugin listens for its database conduit, inside the
    /// container.
    ///
    /// A provider connects to it, and then the traffic runs the other
    /// way: what the plugin writes comes OUT of that socket and goes to
    /// the caller's database, and the answers come back down it. See
    /// [`Postgres`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame::Postgres)
    /// for the pair of channels that carries it.
    ///
    /// So a provider dials in and then serves. Which is the only shape
    /// available: a provider cannot put a listening socket inside
    /// somebody else's container, so if a plugin wants something dialled
    /// FOR it, the plugin is the one that has to be listening.
    pub postgres_port: u16,
    /// Where the plugin listens for its command conduit, inside the
    /// container.
    ///
    /// The same shape as
    /// [`postgres_port`](Self::postgres_port): a provider connects, and
    /// then the plugin asks for commands to be run and the provider
    /// relays them to the caller. See
    /// [`Command`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame::Command).
    pub command_port: u16,
    /// The plugin's configuration.
    ///
    /// Whatever its author defined, and this specification has no
    /// opinion about any of it — a provider passes it through
    /// untouched, and a plugin that receives something it cannot use
    /// is a disagreement between the caller and the plugin's author
    /// that neither the provider nor this field can adjudicate.
    ///
    /// # Why it is not just more environment
    ///
    /// Because the values are JSON, not strings. A plugin taking a
    /// number, a list or a nested object would otherwise have every
    /// caller inventing an encoding for it and every plugin guessing
    /// which one was used.
    ///
    /// Ordered, and the order is preserved deliberately: it is the
    /// caller's order, it survives to the plugin, and two requests
    /// that differ only in it serialize differently.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub arguments: IndexMap<String, serde_json::Value>,
    /// On whose behalf the plugin runs.
    ///
    /// See [`Identity`]. Fixed for the container's life, which is why
    /// it is here rather than on each exchange.
    pub identity: Identity,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 1;

/// JSON, matching the laboratory run this is a variation on.
///
/// One of these is sent per plugin container, so there is no
/// throughput to optimize for — and [`arguments`](Frame::arguments) carries
/// arbitrary JSON, which a positional format could not hold without
/// tunnelling it through a string.
impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        serde_json::from_slice(rest).map_err(FrameError::Body)
    }
}

/// An MCP plugin request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    ///
    /// A reader that dispatched on the tag will not see this. One that
    /// assumed which request it held, and was wrong, will — which is
    /// the point of checking a tag rather than skipping it.
    UnexpectedTag(u8),
    /// The request did not parse.
    Body(serde_json::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("mcp plugin request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(
                    f,
                    "expected mcp plugin request tag {TAG}, found {tag}"
                )
            }
            FrameError::Body(error) => {
                write!(f, "mcp plugin request did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnexpectedTag(_) => None,
        }
    }
}
