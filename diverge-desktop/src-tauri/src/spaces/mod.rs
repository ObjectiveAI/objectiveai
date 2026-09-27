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
//! The wire's types are Ronald's: the SDK's `Container`, `Connect`, `Id`,
//! `Authorize` and its answer, the filetree's `Node`, and `rmcp`'s MCP
//! model at the version his workspace pins. What rides inside them is ours:
//! the room program (`diverge-desktop-room`), the seal on every call, the
//! [`Knocking`] a joiner writes in the opaque authorization, and the
//! [`Invite`] a host hands out.
//!
//! Calls arrive here already sealed by whoever makes them (see
//! [`crate::identity`]); the room checks the seal. [`stub::StubSpaces`] runs
//! the same room program in process until rooms are hosted on the wire.

pub mod stub;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

pub use diverge_sdk::shared::containers::authorize::{request::Authorize, response::Frame as Answer};
pub use diverge_sdk::shared::containers::request::Container;
pub use diverge_sdk::shared::containers::response::Id;
use diverge_desktop_room::{Key, Statement};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::shared::error::Error as WireError;
use diverge_sdk::shared::filetree::response::Node;
use rmcp::model::{CallToolRequestParams, CallToolResult, ErrorData, ListToolsResult, ReadResourceResult, ServerNotification};

use crate::daemon::Frames;

/// One Space this app hosts, or has joined.
#[derive(Debug, Clone)]
pub struct SpaceEntry {
    pub id: Id,
    pub title: String,
    /// What the room's program is: `home`, `board`, `idea`, `dm`, `profile`.
    pub kind: String,
    /// Whose provider runs it, as the daemon names providers.
    pub host: Identity,
    pub host_name: String,
    #[allow(dead_code)] // what a WireSpaces reads from the room; the screen goes by `mine`
    pub host_key: Key,
    pub mine: bool,
    /// A room is only as reachable as its host's connection.
    pub online: bool,
}

/// Someone at the door of a room this app hosts: what the provider
/// observed (the address) and what they wrote (the authorization).
#[derive(Debug, Clone)]
pub struct Knock {
    pub knock_id: u64,
    pub space: Id,
    pub authorize: Authorize,
    pub at: DateTime<Utc>,
}

/// What a joiner writes in the connect's authorization. Opaque to the
/// wire; ours to define: the invite's secret (none for an open knock), the
/// key they'll seal with, the name they'll go by there, a note, whether to
/// be listed, and a member's vouch if they have one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Knocking {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    pub key: Key,
    pub name: String,
    #[serde(default)]
    pub note: String,
    #[serde(default = "listed_by_default")]
    pub listed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vouch: Option<Statement>,
}

fn listed_by_default() -> bool {
    true
}

impl Knocking {
    pub fn to_authorization(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_authorization(authorization: &str) -> Option<Knocking> {
        serde_json::from_str(authorization).ok()
    }
}

/// One verb, as an invite describes it before you're in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InviteVerb {
    pub name: String,
    pub does: String,
}

/// What a host hands out, out of band. The wire needs only the room's id
/// and host; the rest is what the door shows before anyone knocks, and is
/// checked against the room once you're in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invite {
    pub host: Identity,
    pub id: String,
    /// The secret an invited knock presents. None: an open door, knock with a note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    pub title: String,
    pub kind: String,
    pub host_name: String,
    pub charter: String,
    pub verbs: Vec<InviteVerb>,
}

impl Invite {
    const PREFIX: &'static str = "diverge-invite:";

    /// One line of text, to paste anywhere: a DM, a message, a note.
    pub fn to_text(&self) -> String {
        use base64::Engine;
        format!("{}{}", Self::PREFIX, base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(self).unwrap_or_default()))
    }

    pub fn from_text(text: &str) -> Result<Invite, String> {
        use base64::Engine;
        let body = text.trim().strip_prefix(Self::PREFIX).ok_or("that isn't an invite")?;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(body).map_err(|_| "that invite is damaged".to_string())?;
        serde_json::from_slice(&bytes).map_err(|_| "that invite is damaged".to_string())
    }
}

#[derive(Debug, Clone)]
pub enum Joined {
    Joined(Id),
    /// The host said no: `authorize` answered `Denied`.
    Denied,
    /// No such running room: the connect's `{"kind":"missing"}`.
    Missing,
    Error(WireError),
}

/// What a room asks of its host's app: on the wire, the container's own
/// `mcp-call-tool` to its runner.
#[derive(Debug, Clone)]
pub enum HostCall {
    /// A visitor asked one of your agents for something, through your profile.
    Hire { room: Id, hire_id: String, from: String, agent: String, what: String, pledge: Option<String> },
}

#[async_trait]
pub trait Spaces: Send + Sync + 'static {
    async fn list(&self) -> Vec<SpaceEntry>;

    /// `containers::tools::run` on this app's own provider: host a room.
    /// The room program's `Args` travel in the container's `arguments`.
    async fn host(&self, container: Container) -> Result<Id, WireError>;

    /// Everyone at the door of rooms this app hosts: the pending `authorize`
    /// asks, then each new one as it comes, until `cancel`.
    fn knocks(&self, cancel: CancellationToken) -> Frames<Knock>;

    /// The host's answer on the `authorize` channel. Letting someone in is
    /// then the host's `admit`, sealed like any call.
    async fn answer(&self, knock_id: u64, answer: Answer) -> Result<Knock, String>;

    /// `containers::tools::connect` to a room someone else hosts, presenting
    /// a [`Knocking`]. Resolves when the host answers.
    async fn join(&self, invite: &Invite, knocking: &Knocking) -> Joined;

    /// The connect scope's `disconnect`; for a room this app hosts, the run ends.
    async fn leave(&self, id: &Id) -> Result<(), String>;

    async fn tools(&self, id: &Id) -> Result<ListToolsResult, ErrorData>;
    async fn read(&self, id: &Id, uri: &str) -> Result<ReadResourceResult, ErrorData>;
    /// A sealed call. The room checks the seal; nothing here says who is calling.
    async fn call(&self, id: &Id, params: CallToolRequestParams) -> Result<CallToolResult, ErrorData>;
    fn notifications(&self, id: &Id, cancel: CancellationToken) -> Frames<ServerNotification>;

    /// What to hand a friend so they can knock: an invite with its secret.
    async fn invite(&self, id: &Id) -> Option<Invite>;

    /// Your own Home room and your profile room, if you host them.
    async fn home(&self) -> Option<Id>;
    async fn profile(&self) -> Option<Id>;

    /// What rooms you host ask of you, as they ask it.
    fn host_calls(&self, cancel: CancellationToken) -> Frames<HostCall>;

    /// The table: the room container's files, reached as any member reaches
    /// them (`filetree`, `read`, `write`), and `transfer` between two rooms
    /// on one provider.
    async fn table_tree(&self, id: &Id) -> Result<Vec<Node>, WireError>;
    async fn table_read(&self, id: &Id, path: &[String]) -> Result<Vec<u8>, WireError>;
    async fn table_write(&self, id: &Id, path: &[String], body: Vec<u8>) -> Result<(), WireError>;
    async fn transfer(&self, from: &Id, path: &[String], to: &Id) -> Result<(), WireError>;

    /// Stop a room you host and run it again from its record: the run
    /// scope's `stop`, then `containers::tools::run`. Everyone must connect
    /// again; the files are closed to anyone the room no longer admits.
    async fn restart(&self, id: &Id) -> Result<(), WireError>;
}
