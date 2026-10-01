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
//! [`crate::identity`]); the room checks the seal. `stub::StubSpaces`
//! (feature `stand-in`) runs the same room program in process until rooms
//! are hosted on the wire.

#[cfg(feature = "stand-in")]
pub mod stub;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

pub use diverge_sdk::shared::containers::authorize::{request::Authorize, response::Frame as Answer};
pub use diverge_sdk::shared::containers::request::Container;
pub use diverge_sdk::shared::containers::response::Id;
use diverge_desktop_room::account::Proof;
use diverge_desktop_room::seal::{digest, statement_holds};
use diverge_desktop_room::{Key, Keypair, Statement};
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Knock {
    pub knock_id: u64,
    pub space: Id,
    pub authorize: Authorize,
    pub at: DateTime<Utc>,
}

/// What a joiner writes in the connect's authorization. Opaque to the wire,
/// ours to define, and signed by the key it names, so the host can check
/// that whoever knocked holds that key: the room it's for, a mark of the
/// invite it came with (none for an open knock), the name they'll go by,
/// a note, whether to be listed, a member's vouch if they have one, their
/// account (a room under rules 2 lets a person in by it), and when.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Knocking {
    /// The room knocked at. A knock is good there only.
    pub room: String,
    /// The invite it came with, as a mark the host who made the invite can check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invite: Option<String>,
    pub key: Key,
    pub name: String,
    #[serde(default)]
    pub note: String,
    #[serde(default = "listed_by_default")]
    pub listed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vouch: Option<Statement>,
    /// The knocker's account: its genesis and newest device list, which
    /// names `key`. Left out by a knock from before accounts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<Proof>,
    pub at: DateTime<Utc>,
    /// `key`'s signature over everything above.
    #[serde(default)]
    pub sig: String,
}

fn listed_by_default() -> bool {
    true
}

/// How long a knock is good for.
const KNOCK_GOOD_FOR: TimeDelta = TimeDelta::hours(24);

/// How long a vouch is good for, from when it's made.
pub const VOUCH_GOOD_FOR: TimeDelta = TimeDelta::days(7);

impl Knocking {
    /// What a knock carries for an invite: never the secret itself, which only
    /// the room's host and the invited hold.
    pub fn invite_mark(room: &str, secret: &str) -> String {
        digest(format!("diverge-desktop invite\n{room}\n{secret}").as_bytes())[..32].to_owned()
    }

    /// Everything the knocker signs.
    pub fn body(&self) -> serde_json::Value {
        let mut v = serde_json::to_value(self).unwrap_or_default();
        if let Some(o) = v.as_object_mut() {
            o.remove("sig");
        }
        v
    }

    /// Signed with a key held directly: the stand-in's people and tests. The
    /// app signs through its keys instead ([`Knocking::body`], then `sig`).
    pub fn signed(mut self, keypair: &Keypair) -> Self {
        self.sig = Statement::make(keypair, "knock", self.body()).sig;
        self
    }

    /// Whether this is a knock at `room`, signed by the key it names, made lately.
    pub fn check(&self, room: &str, now: DateTime<Utc>) -> Result<(), &'static str> {
        if self.room != room {
            return Err("made for another room");
        }
        if !statement_holds(&self.key, "knock", &self.body(), &self.sig) {
            return Err("not signed by the key it names");
        }
        if self.at > now + TimeDelta::minutes(5) || now - self.at > KNOCK_GOOD_FOR {
            return Err("too old to trust");
        }
        Ok(())
    }

    /// The knocker's account, if the knock carries one that holds and names
    /// the key that signed the knock.
    pub fn account_id(&self) -> Option<String> {
        self.account.as_ref().filter(|p| p.names(&self.key)).map(Proof::id)
    }

    /// Whether a vouch holds for whoever knocked: for their key, or for the account the knock carries.
    pub fn vouched(&self, vouch: &Statement, room: &str, now: DateTime<Utc>) -> bool {
        vouch_holds(vouch, &self.key, room, now) || self.account_id().is_some_and(|a| vouch_holds(vouch, &a, room, now))
    }

    pub fn to_authorization(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_authorization(authorization: &str) -> Option<Knocking> {
        serde_json::from_str(authorization).ok()
    }
}

/// A vouch: a member's word for someone's key, into one room, until a time.
/// It holds only for that key, that room, and until then.
pub fn vouch_holds(vouch: &Statement, for_key: &str, room: &str, now: DateTime<Utc>) -> bool {
    vouch.kind == "vouch"
        && vouch.holds()
        && vouch.field("for") == Some(for_key)
        && vouch.field("room") == Some(room)
        && vouch.field("until").and_then(|u| DateTime::parse_from_rfc3339(u).ok()).is_some_and(|u| u.with_timezone(&Utc) > now)
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
    /// A knock still waiting at the door, without answering it.
    async fn pending(&self, knock_id: u64) -> Option<Knock>;

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
    /// again; the files are closed to anyone the room no longer admits. The
    /// invite stays as it was.
    async fn restart(&self, id: &Id) -> Result<(), WireError>;

    /// The host's restart to shut someone out: [`Spaces::restart`], and the
    /// room gets a new invite secret, so a knock with an invite handed out
    /// before no longer comes with this room's invite. Returns the new invite.
    async fn restart_with_new_invite(&self, id: &Id) -> Result<Invite, WireError>;
}
