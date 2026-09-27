//! The Spaces seam: the social layer, built on the provider protocol's
//! tool rooms.
//!
//! A Space is a *tool container* a host runs on their own provider
//! (`containers::tools::run`) with a persistent volume. Another person
//! joins it by id, presenting an invite (`containers::tools::connect`);
//! the provider asks the host on the run's `authorize` channel, and the
//! host's app answers yes or no. Inside, a member lists and calls the
//! room's MCP tools, reads its resources and hears its notifications —
//! the same exchanges an agent has with the caller's MCP.
//!
//! Every type on this seam is Ronald's: the provider SDK's `Container`,
//! `Connect`, `Id`, `Authorize` and its answer, and `rmcp`'s MCP model at
//! the version his workspace pins. [`stub::StubSpaces`] keeps rooms in
//! process until his daemon relays these;
//! a `WireSpaces` over the provider SDK's `client` half replaces it.

pub mod stub;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;

pub use diverge_sdk::shared::containers::authorize::{request::Authorize, response::Frame as Answer};
pub use diverge_sdk::shared::containers::request::{Connect, Container};
pub use diverge_sdk::shared::containers::response::Id;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::shared::error::Error as WireError;
use rmcp::model::{CallToolRequestParams, CallToolResult, ErrorData, ListToolsResult, ReadResourceResult, ServerNotification};

use crate::daemon::Frames;

/// One Space this daemon hosts, or has joined.
#[derive(Debug, Clone)]
pub struct SpaceEntry {
    pub id: Id,
    pub title: String,
    /// What the room's image is: `home`, `board`, `idea`, `dm`.
    pub kind: String,
    /// Whose provider runs it, as the daemon names providers.
    pub host: Identity,
    pub mine: bool,
    /// A room is only as reachable as its host's machine.
    pub online: bool,
    /// The name this daemon's person is known by inside.
    pub joined_as: String,
}

/// Someone at the door of a room this daemon hosts: what the provider
/// observed (the address) and what they presented (the authorization).
#[derive(Debug, Clone)]
pub struct Knock {
    pub knock_id: u64,
    pub space: Id,
    pub authorize: Authorize,
    pub at: DateTime<Utc>,
}

/// What a host hands out: where the room is, and what to present.
#[derive(Debug, Clone, PartialEq)]
pub struct Invite {
    pub host: Identity,
    pub connect: Connect,
}

#[derive(Debug, Clone)]
pub enum Joined {
    Joined(Id),
    /// The host said no — `authorize` answered `Denied`.
    Denied,
    /// No such running room — the connect's `{"kind":"missing"}`.
    Missing,
    Error(WireError),
}

/// Who is calling a room's tool: the person at the keyboard, or one of
/// their agents through the agent door.
#[derive(Debug, Clone)]
pub enum Caller {
    Person,
    Agent(String),
}

#[async_trait]
pub trait Spaces: Send + Sync + 'static {
    async fn list(&self) -> Vec<SpaceEntry>;

    /// `containers::tools::run` on this daemon's own provider: host a room.
    /// The room's title, kind, charter and invite key travel in the
    /// container's `arguments`, as any tool's settings do.
    async fn host(&self, container: Container) -> Result<Id, WireError>;

    /// Everyone at the door of rooms this daemon hosts: the pending
    /// `authorize` asks, then each new one as it comes, until `cancel`.
    fn knocks(&self, cancel: CancellationToken) -> Frames<Knock>;

    /// The host's answer on the `authorize` channel.
    async fn answer(&self, knock_id: u64, answer: Answer) -> Result<(), String>;

    /// `containers::tools::connect` to a room someone else hosts.
    async fn join(&self, invite: Invite, as_name: String) -> Joined;

    /// The connect scope's `disconnect`; for a room this daemon hosts, the run ends.
    async fn leave(&self, id: &Id) -> Result<(), String>;

    async fn tools(&self, id: &Id) -> Result<ListToolsResult, ErrorData>;
    async fn read(&self, id: &Id, uri: &str) -> Result<ReadResourceResult, ErrorData>;
    async fn call(&self, id: &Id, params: CallToolRequestParams, caller: Caller) -> Result<CallToolResult, ErrorData>;
    fn notifications(&self, id: &Id, cancel: CancellationToken) -> Frames<ServerNotification>;

    /// What to hand a friend so they can knock.
    async fn invite(&self, id: &Id) -> Option<Invite>;

    /// This person's own Home room, if they host one.
    async fn home(&self) -> Option<Id>;
}
