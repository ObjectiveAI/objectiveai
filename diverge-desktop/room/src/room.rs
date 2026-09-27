//! One room: what a Space's tool container runs.
//!
//! Its verbs are MCP tools, its objects are *moves*, and every change is an
//! MCP resource-updated notification, which the wire fans out to every
//! member. What makes it a room people can trust:
//!
//! - **Its settings are its host's.** The host signs them, and the room's id
//!   ends in the host's mark, so no other host can run a room by that id.
//! - **Every call is sealed** (see [`crate::seal`]) for this room's id. The
//!   room checks the seal against the keys its host admitted, and refuses
//!   anything else.
//! - **Every move is chained and countersigned**: it carries the hash of the
//!   move before it, the verb, arguments and seal that made it, and the
//!   room's own signature over all of that. The room's key is one the host's
//!   app made for this room and named in the signed settings.
//! - **A record is checked by replaying it**, under the same rules a live
//!   call meets. Whatever a move says (its title, its body, its fields) must
//!   be what its sealed verb and arguments make. A restart, a successor, and
//!   a member checking their copy all take this one path.
//! - **Moves never change.** What changes (a task claimed, a hire taken) is
//!   derived from later moves.
//! - **Receipts are sealed by the host**, so a receipt proves which room
//!   issued it wherever it is shown.
//!
//! One thing no record can show is what came after it: a copy that stops
//! early is a true copy of an earlier moment. Copies say when they end.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ContentBlock, ErrorData, JsonObject, ListToolsResult, MetaObject, Notification, ReadResourceResult,
    ResourceContents, ResourceUpdatedNotificationParam, ServerNotification, Tool,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tokio::sync::broadcast;

use crate::seal::{Key, Keypair, Seal, Statement, canonical, check_call, countersigned, digest, fingerprint, id_holds, recheck, statement_holds};

pub const FEED: &str = "space://feed";
pub const CHARTER: &str = "space://charter";
pub const CHARTERS: &str = "space://charters";
pub const MEMBERS: &str = "space://members";
pub const DOORWAYS: &str = "space://doorways";
pub const ABOUT: &str = "space://about";
pub const RECORD: &str = "space://record";

/// The `_meta` key that marks a verb only the host may use.
pub const META_HOST_ONLY: &str = "network.diverge.desktop/host_only";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Home,
    Board,
    Idea,
    Dm,
    Profile,
}

impl Kind {
    pub const ALL: [Kind; 5] = [Kind::Home, Kind::Board, Kind::Idea, Kind::Dm, Kind::Profile];

    pub fn key(self) -> &'static str {
        match self {
            Kind::Home => "home",
            Kind::Board => "board",
            Kind::Idea => "idea",
            Kind::Dm => "dm",
            Kind::Profile => "profile",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.key() == s)
    }
}

/// What a host starts a room with: the container's `arguments`, signed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Args {
    /// A label and the host's mark: see [`crate::seal::room_id`].
    pub id: String,
    pub title: String,
    pub kind: Kind,
    pub host_key: Key,
    pub host_name: String,
    pub charter: String,
    /// Whether someone may knock without an invite, with a note.
    #[serde(default)]
    pub open_door: bool,
    /// The room this one continues, when a member starts a successor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continues: Option<Continues>,
    /// The key the room countersigns its moves with. The host's app made it
    /// for this room; the room program holds its secret.
    pub room_key: Key,
    /// When the host made the room.
    pub at: DateTime<Utc>,
    /// The host's signature over everything above.
    #[serde(default)]
    pub sig: String,
}

impl Args {
    /// Everything the host signs: the settings without the signature.
    pub fn body(&self) -> Value {
        let mut v = serde_json::to_value(self).unwrap_or_default();
        if let Some(o) = v.as_object_mut() {
            o.remove("sig");
        }
        v
    }

    /// Signed by the host's key directly: the stand-in and tests. The app signs
    /// through its keys instead ([`Args::body`], then set `sig`).
    pub fn signed(mut self, host: &Keypair) -> Self {
        self.sig = Statement::make(host, "room", self.body()).sig;
        self
    }

    /// Whether these are settings their host made: the id is theirs and they signed it all.
    pub fn holds(&self) -> Result<(), String> {
        if !id_holds(&self.id, &self.host_key) {
            return Err(format!("{} isn't an id its host could have made", self.id));
        }
        if !statement_holds(&self.host_key, "room", &self.body(), &self.sig) {
            return Err("this room's settings aren't signed by its host".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Continues {
    pub room: String,
    pub title: String,
    /// The hash of the last move of the room it continues.
    pub last: String,
}

/// A room's whole record: its settings, the record of the room it continues
/// (if it does), and its moves. Enough to check it, restart it, or continue it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub args: Args,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Box<Record>>,
    #[serde(default)]
    pub moves: Vec<Move>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub key: Key,
    pub name: String,
    pub is_agent: bool,
    /// The person an agent acts for.
    pub agent_of: Option<Key>,
    pub agent_of_name: Option<String>,
    /// Whether they chose to be on the room's list. The wire reports nobody.
    pub listed: bool,
    pub joined: DateTime<Utc>,
    pub removed: bool,
}

/// One move, as it was made. Never changed afterwards.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Move {
    pub seq: u64,
    pub id: String,
    pub kind: String,
    /// The key that made it.
    pub by: Key,
    /// The name that key was admitted under.
    pub author: String,
    /// For an agent: the name of the person it acts for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_of: Option<String>,
    pub at: DateTime<Utc>,
    pub title: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default)]
    pub fields: Map<String, Value>,
    /// The charter in force when it was made.
    pub charter: String,
    /// What was sealed: the verb, its arguments, and the seal.
    pub verb: String,
    pub args: JsonObject,
    pub seal: Seal,
    pub prev: String,
    pub hash: String,
    /// The room's countersign over `hash`.
    #[serde(default)]
    pub room_sig: String,
}

impl Move {
    fn content_hash(&self) -> String {
        let mut v = serde_json::to_value(self).unwrap_or_default();
        if let Some(o) = v.as_object_mut() {
            o.remove("hash");
            o.remove("room_sig");
        }
        digest(canonical(&v).as_bytes())
    }
}

/// What the host's side does for a room: seal its receipts, hear its hires.
/// In-process, the app itself; on the wire, the room's calls to its runner.
pub trait Host: Send + Sync {
    fn seal(&self, kind: &str, body: Value) -> Result<Statement, String>;
    fn hire(&self, _room: &str, _hire_id: &str, _from: &str, _agent: &str, _what: &str, _pledge: Option<&str>) {}
}

/// No host at all: for replaying a record, which never asks one.
pub struct NoHost;

impl Host for NoHost {
    fn seal(&self, _kind: &str, _body: Value) -> Result<Statement, String> {
        Err("a copy of a room seals nothing".into())
    }
}

#[derive(Clone)]
struct Charter {
    fingerprint: String,
    text: String,
    at: DateTime<Utc>,
}

pub struct Room {
    pub args: Args,
    /// The room's own key, when this is the room itself; `None` for a copy
    /// someone is checking, which can't make new moves.
    room_key: Option<Keypair>,
    charters: Vec<Charter>,
    members: IndexMap<Key, Member>,
    moves: Vec<Move>,
    /// What later moves made of earlier ones: id → (state, fields).
    derived: HashMap<String, (String, Map<String, Value>)>,
    counters: HashMap<Key, u64>,
    /// The room this one continues, rebuilt from its record by replay.
    old: Option<Box<Room>>,
    live: broadcast::Sender<ServerNotification>,
}

fn schema(props: Value, required: &[&str]) -> Arc<JsonObject> {
    let mut o = JsonObject::new();
    o.insert("type".into(), json!("object"));
    o.insert("properties".into(), props);
    o.insert("required".into(), json!(required));
    Arc::new(o)
}

fn verb(name: &'static str, does: &'static str, props: Value, required: &[&str]) -> Tool {
    Tool::new(name, does, schema(props, required))
}

fn host_only(tool: Tool) -> Tool {
    let mut meta = JsonObject::new();
    meta.insert(META_HOST_ONLY.into(), json!(true));
    tool.with_meta(MetaObject(meta))
}

fn bad(message: impl Into<String>) -> ErrorData {
    ErrorData::invalid_params(message.into(), None)
}

fn refused(message: impl Into<String>) -> ErrorData {
    ErrorData::invalid_request(message.into(), None)
}

fn arg<'a>(args: &'a JsonObject, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str).filter(|s| !s.trim().is_empty())
}

fn need<'a>(args: &'a JsonObject, key: &str) -> Result<&'a str, ErrorData> {
    arg(args, key).ok_or_else(|| bad(format!("{key} is needed")))
}

/// The state a move starts in.
fn first_state(kind: &str) -> &'static str {
    match kind {
        "show" => "shown",
        "ask" | "task" | "direction" | "synthesis" => "open",
        "offering" | "offer" => "offered",
        "claim" => "claimed",
        "delivery" | "hire_delivery" => "delivered",
        "receipt" => "issued",
        "run" => "done",
        "hire" => "asked",
        "note" => "left",
        _ => "said",
    }
}

impl Room {
    /// A new room, made by its host with the key they made for it.
    pub fn new(args: Args, room_key: Keypair) -> Result<Room, String> {
        Self::open(args, Some(room_key), None)
    }

    /// A room from its settings, before any move: checks the settings, and
    /// for a successor, replays the record it continues and checks that
    /// whoever starts it was still in that room.
    pub fn open(args: Args, room_key: Option<Keypair>, before: Option<Record>) -> Result<Room, String> {
        args.holds()?;
        if let Some(k) = &room_key {
            if k.key() != args.room_key {
                return Err("that isn't this room's key".into());
            }
        }
        let old = match (&args.continues, before) {
            (None, None) => None,
            (Some(c), Some(b)) => {
                if b.args.id != c.room {
                    return Err("that record is of another room".into());
                }
                let old = Room::check(&b).map_err(|e| format!("the room it continues doesn't check: {e}"))?;
                if old.last_hash() != c.last {
                    return Err("that record doesn't end where the room it continues ended".into());
                }
                let still_in = args.host_key == b.args.host_key || old.members.get(&args.host_key).is_some_and(|m| !m.removed && !m.is_agent);
                if !still_in {
                    return Err("only someone still in a room may continue it".into());
                }
                Some(Box::new(old))
            }
            (Some(_), None) => return Err("a room that continues another needs that room's record".into()),
            (None, Some(_)) => return Err("a record came with a room that continues nothing".into()),
        };
        let (live, _) = broadcast::channel(256);
        let charter = Charter { fingerprint: fingerprint(&args.charter), text: args.charter.clone(), at: args.at };
        let mut members = IndexMap::new();
        members.insert(
            args.host_key.clone(),
            Member { key: args.host_key.clone(), name: args.host_name.clone(), is_agent: false, agent_of: None, agent_of_name: None, listed: true, joined: args.at, removed: false },
        );
        Ok(Room { args, room_key, charters: vec![charter], members, moves: Vec::new(), derived: HashMap::new(), counters: HashMap::new(), old, live })
    }

    /// A room rebuilt from its record, every move replayed under the rules a
    /// live call meets. With the room's key, it's the room again (a restart);
    /// without, a checked copy.
    pub fn from_record(record: Record, room_key: Option<Keypair>) -> Result<Room, String> {
        let Record { args, before, moves } = record;
        let mut room = Room::open(args, room_key, before.map(|b| *b))?;
        for m in &moves {
            room.replay(m)?;
        }
        Ok(room)
    }

    /// Check a record someone holds: the whole of it, by replay.
    pub fn check(record: &Record) -> Result<Room, String> {
        Room::from_record(record.clone(), None)
    }

    pub fn record(&self) -> Record {
        Record { args: self.args.clone(), before: self.old.as_ref().map(|o| Box::new(o.record())), moves: self.moves.clone() }
    }

    pub fn id(&self) -> &str {
        &self.args.id
    }

    pub fn charter_fingerprint(&self) -> &str {
        self.charters.last().map(|c| c.fingerprint.as_str()).unwrap_or_default()
    }

    pub fn charter(&self) -> &str {
        self.charters.last().map(|c| c.text.as_str()).unwrap_or_default()
    }

    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    pub fn members(&self) -> impl Iterator<Item = &Member> {
        self.members.values()
    }

    pub fn member(&self, key: &str) -> Option<&Member> {
        self.members.get(key)
    }

    pub fn last_hash(&self) -> String {
        self.moves.last().map(|m| m.hash.clone()).unwrap_or_default()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerNotification> {
        self.live.subscribe()
    }

    fn notify(&self, uri: &str) {
        let _ = self.live.send(ServerNotification::ResourceUpdatedNotification(Notification::new(ResourceUpdatedNotificationParam::new(uri))));
    }

    /// The room's verbs, as MCP tools: what a person clicks and an agent
    /// calls. Host-only verbs say so under `_meta`; the room enforces it.
    pub fn tools(&self) -> ListToolsResult {
        let kind = self.args.kind;
        let mut tools = Vec::new();
        let show = verb("show", "Show something you made or did. What you show stays as shown; a new version is a new show.", json!({ "title": { "type": "string" }, "body": { "type": "string", "description": "What it is, in your words." } }), &["title"]);
        let ask = verb(
            "ask",
            "Ask for something. Anyone here, a person or an agent, may offer to serve it.",
            json!({
                "what": { "type": "string", "description": "What you need." },
                "needs": { "type": "string", "description": "What serving it takes: skills, access, hardware." },
                "ceiling": { "type": "string", "description": "The most you'd spend on it: time, tokens, or a budget." },
                "who_may_serve": { "type": "string", "enum": ["anyone", "members", "agents"], "description": "Who may pick it up." },
                "thread": { "type": "string", "description": "Set by the app when one ask goes to several rooms." }
            }),
            &["what"],
        );
        let offer = verb("offer", "Offer to serve an ask.", json!({ "ask_id": { "type": "string" }, "body": { "type": "string", "description": "What you'd do, and on what terms." } }), &["ask_id", "body"]);
        let reply = verb("reply", "Reply to a move.", json!({ "move_id": { "type": "string" }, "body": { "type": "string" } }), &["move_id", "body"]);
        let offering = verb(
            "post_offering",
            "Offer something you sell or lend: a fixed thing, or a metered capability.",
            json!({
                "title": { "type": "string" },
                "what": { "type": "string" },
                "pricing": { "type": "string", "enum": ["fixed", "metered"], "description": "Fixed: one price. Metered: priced in what the wire counts." },
                "terms": { "type": "string" }
            }),
            &["title", "what", "pricing"],
        );
        match kind {
            Kind::Dm => {
                tools.push(verb("say", "Say something.", json!({ "body": { "type": "string" } }), &["body"]));
                tools.push(reply);
            }
            Kind::Profile => {
                tools.push(host_only(show));
                tools.push(host_only(offering.clone()));
                tools.push(host_only(verb("pin_receipt", "Show a receipt you were issued elsewhere. It keeps the seal of the room that issued it.", json!({ "statement": { "type": "object" } }), &["statement"])));
                tools.push(verb("leave_note", "Leave a note for whoever this profile belongs to.", json!({ "body": { "type": "string" } }), &["body"]));
                tools.push(verb(
                    "hire",
                    "Ask one of this person's agents to do something for you. They decide; their agent does the work on their machine.",
                    json!({ "agent": { "type": "string" }, "what": { "type": "string" }, "pledge": { "type": "string", "description": "What you'll give for it, in words. Nothing moves through this room." } }),
                    &["agent", "what"],
                ));
                tools.push(host_only(verb("answer_hire", "Take a hire, or turn it down.", json!({ "hire_id": { "type": "string" }, "take": { "type": "boolean" }, "note": { "type": "string" } }), &["hire_id", "take"])));
                tools.push(host_only(verb("deliver_hire", "Deliver what a hire asked for.", json!({ "hire_id": { "type": "string" }, "summary": { "type": "string" }, "files": { "type": "array", "items": { "type": "string" }, "description": "Paths on this room's table." } }), &["hire_id", "summary"])));
                tools.push(reply);
            }
            _ => {
                tools.push(show);
                tools.push(ask);
                tools.push(offer);
                tools.push(reply);
            }
        }
        match kind {
            Kind::Home => tools.push(verb("report", "Report what you finished: for an agent, when a run ends.", json!({ "title": { "type": "string" }, "body": { "type": "string" }, "measured": { "type": "string", "description": "What the run measured: tokens, seconds." } }), &["title"])),
            Kind::Board => tools.extend([
                verb(
                    "post_task",
                    "Post a task: its spec is fixed at posting, and whoever claims it is judged on the spec.",
                    json!({
                        "title": { "type": "string" },
                        "spec": { "type": "string", "description": "What done looks like. The spec is what gets checked, not the hours." },
                        "pledge": { "type": "string", "description": "What you'll give for it, in words. Nothing moves through this room; both sides say when it's settled." }
                    }),
                    &["title", "spec"],
                ),
                verb("claim", "Claim an open task. You take it as posted.", json!({ "task_id": { "type": "string" } }), &["task_id"]),
                verb(
                    "deliver",
                    "Deliver on a task you claimed.",
                    json!({ "task_id": { "type": "string" }, "summary": { "type": "string", "description": "What you did, against the spec." }, "files": { "type": "array", "items": { "type": "string" }, "description": "Paths on this room's table." } }),
                    &["task_id", "summary"],
                ),
                verb("accept", "Accept a delivery: the task is done and the room issues a receipt.", json!({ "task_id": { "type": "string" } }), &["task_id"]),
                verb(
                    "settle",
                    "Say whether a task's pledge is settled. Whoever posted it and whoever did it each say; if they disagree, both stand.",
                    json!({ "task_id": { "type": "string" }, "agree": { "type": "boolean", "description": "Yes: settled as pledged." }, "note": { "type": "string" } }),
                    &["task_id", "agree"],
                ),
                offering,
            ]),
            Kind::Idea => tools.extend([
                verb("propose", "Propose a direction the idea could go.", json!({ "direction": { "type": "string" }, "body": { "type": "string" } }), &["direction"]),
                verb(
                    "steer",
                    "Steer a direction: prefer it, reject it, or leave a note.",
                    json!({ "direction_id": { "type": "string" }, "move": { "type": "string", "enum": ["prefer", "reject", "note"] }, "note": { "type": "string" } }),
                    &["direction_id", "move"],
                ),
                verb("synthesize", "Write where the idea has landed so far. The next round starts from here.", json!({ "body": { "type": "string" } }), &["body"]),
            ]),
            Kind::Dm | Kind::Profile => {}
        }
        // The host's own verbs.
        tools.push(host_only(verb(
            "admit",
            "Let someone in: a person you said yes to at the door, or an agent tethered to a member.",
            json!({
                "key": { "type": "string" },
                "name": { "type": "string" },
                "is_agent": { "type": "boolean" },
                "agent_of": { "type": "string", "description": "For an agent: its person's key." },
                "tether": { "type": "object", "description": "For an agent: its person's statement that the key is theirs." },
                "listed": { "type": "boolean", "description": "Whether they asked to be on the room's list." }
            }),
            &["key", "name"],
        )));
        tools.push(host_only(verb("remove", "Remove someone: the room refuses their calls from now on.", json!({ "key": { "type": "string" }, "reason": { "type": "string" } }), &["key"])));
        tools.push(host_only(verb("set_charter", "Change the room's rules. Moves keep the version they were made under.", json!({ "text": { "type": "string" } }), &["text"])));
        if kind != Kind::Dm {
            tools.push(host_only(verb("vouch_room", "List a room this one vouches for, with an invite to it.", json!({ "title": { "type": "string" }, "invite": { "type": "string" } }), &["title", "invite"])));
        }
        ListToolsResult::with_all_items(tools)
    }

    fn has_verb(&self, name: &str) -> Option<bool> {
        self.tools().tools.iter().find(|t| t.name == name).map(|t| t.meta.as_ref().and_then(|m| m.0.get(META_HOST_ONLY)).and_then(Value::as_bool).unwrap_or(false))
    }

    /// A move as served: what it was made as, plus what later moves made of it.
    fn served(&self, m: &Move, from: Option<&str>) -> Value {
        let (state, extra) = self.derived.get(&m.id).cloned().unwrap_or_else(|| (first_state(&m.kind).to_owned(), Map::new()));
        let mut fields = m.fields.clone();
        fields.extend(extra);
        if let Some(from) = from {
            fields.insert("from".into(), json!(from));
        }
        json!({
            "id": m.id, "seq": m.seq, "kind": m.kind, "author": m.author, "by": m.by, "agent_of": m.agent_of,
            "at": m.at, "title": m.title, "body": m.body, "state": state, "parent": m.parent, "fields": fields,
            "charter": m.charter, "hash": m.hash,
        })
    }

    pub fn read(&self, uri: &str) -> Result<ReadResourceResult, ErrorData> {
        let (text, mime) = match uri {
            FEED => {
                // Someone who asked not to be listed isn't announced; the record still holds it.
                let shown = |m: &&Move| !(m.kind == "admitted" && m.args.get("listed").and_then(Value::as_bool) == Some(false));
                // The room it continues, as that room had it: a task finished there reads finished here.
                let history: Vec<Value> = match &self.old {
                    Some(old) => old.moves.iter().filter(shown).map(|m| old.served(m, Some(&old.args.title))).collect(),
                    None => Vec::new(),
                };
                let all: Vec<Value> = history.into_iter().chain(self.moves.iter().filter(shown).map(|m| self.served(m, None))).collect();
                (serde_json::to_string(&all).unwrap_or_default(), "application/json")
            }
            MEMBERS => {
                let listed: Vec<Value> = self
                    .members
                    .values()
                    // An agent is listed only if its person is: listing it would name them.
                    .filter(|m| m.listed && !m.removed && m.agent_of.as_ref().is_none_or(|p| self.members.get(p).is_some_and(|o| o.listed)))
                    .map(|m| {
                        let last = self.moves.iter().rev().find(|x| x.by == m.key).map(|x| x.at);
                        json!({ "name": m.name, "key": m.key, "is_agent": m.is_agent, "agent_of": m.agent_of_name, "joined": m.joined, "last_acted": last })
                    })
                    .collect();
                (serde_json::to_string(&listed).unwrap_or_default(), "application/json")
            }
            CHARTER => (self.charter().to_owned(), "text/markdown"),
            CHARTERS => (
                serde_json::to_string(&self.charters.iter().map(|c| json!({ "fingerprint": c.fingerprint, "at": c.at })).collect::<Vec<_>>()).unwrap_or_default(),
                "application/json",
            ),
            DOORWAYS => {
                let doorways: Vec<Value> = self.moves.iter().filter(|m| m.kind == "doorway").map(|m| json!({ "title": m.title, "invite": m.body, "by": m.author, "at": m.at })).collect();
                (serde_json::to_string(&doorways).unwrap_or_default(), "application/json")
            }
            ABOUT => (
                json!({
                    "id": self.args.id, "title": self.args.title, "kind": self.args.kind, "host_name": self.args.host_name, "host_key": self.args.host_key,
                    "charter": self.charter_fingerprint(), "open_door": self.args.open_door, "continues": self.args.continues,
                    "room_key": self.args.room_key, "at": self.args.at,
                })
                .to_string(),
                "application/json",
            ),
            RECORD => (serde_json::to_string(&self.record()).unwrap_or_default(), "application/json"),
            _ => return Err(ErrorData::resource_not_found(format!("no resource at {uri}"), None)),
        };
        Ok(ReadResourceResult::new(vec![ResourceContents::TextResourceContents { uri: uri.into(), mime_type: Some(mime.into()), text, meta: None }]))
    }

    /// A sealed call, now.
    pub fn call(&mut self, params: CallToolRequestParams, host: &dyn Host) -> Result<CallToolResult, ErrorData> {
        self.call_at(params, Utc::now(), host)
    }

    /// A sealed call at a given time: the stand-in seeds history this way.
    pub fn call_at(&mut self, params: CallToolRequestParams, at: DateTime<Utc>, host: &dyn Host) -> Result<CallToolResult, ErrorData> {
        let name = params.name.to_string();
        self.has_verb(&name).ok_or_else(|| bad(format!("this room has no verb called {name}")))?;
        let seal = check_call(&self.args.id, &params).map_err(refused)?;
        let who = self.gate(&name, &seal)?;
        let args = params.arguments.clone().unwrap_or_default();
        let line = self.apply(&name, &args, &seal, &who, at, host, None)?;
        self.counters.insert(seal.key.clone(), seal.counter);
        self.notify(FEED);
        Ok(CallToolResult::success(vec![ContentBlock::text(line)]))
    }

    /// Whether this sealed key may use this verb now: the rules every call
    /// meets, live or replayed.
    fn gate(&self, name: &str, seal: &Seal) -> Result<Member, ErrorData> {
        let host_verb = self.has_verb(name).ok_or_else(|| bad(format!("this room has no verb called {name}")))?;
        if seal.counter <= self.counters.get(&seal.key).copied().unwrap_or(0) {
            return Err(refused("this call was already made"));
        }
        let who = match self.members.get(&seal.key) {
            Some(m) if m.removed => return Err(refused(format!("{} was removed from this room", m.name))),
            Some(m) => m.clone(),
            None => return Err(refused("that key is not a member here")),
        };
        if host_verb && seal.key != self.args.host_key {
            return Err(refused("only the host may do that"));
        }
        Ok(who)
    }

    /// One move from a record, replayed: its seal re-checked, the call gated
    /// as it would be live, and what it makes compared with what it says.
    fn replay(&mut self, m: &Move) -> Result<(), String> {
        if m.seal.key != m.by || !recheck(&self.args.id, &m.verb, &m.args, &m.seal) {
            return Err(format!("{}'s seal does not hold", m.id));
        }
        let who = self.gate(&m.verb, &m.seal).map_err(|e| format!("{} would have been refused: {}", m.id, e.message))?;
        self.apply(&m.verb, &m.args, &m.seal, &who, m.at, &NoHost, Some(m)).map_err(|e| format!("{} does not replay: {}", m.id, e.message))?;
        self.counters.insert(m.seal.key.clone(), m.seal.counter);
        if self.moves.last() != Some(m) {
            return Err(format!("{} was changed after it was made", m.id));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        at: DateTime<Utc>,
        who: &Member,
        verb: &str,
        args: &JsonObject,
        seal: &Seal,
        kind: &str,
        title: &str,
        body: &str,
        parent: Option<&str>,
        fields: Map<String, Value>,
        replaying: Option<&Move>,
    ) -> Result<String, ErrorData> {
        let seq = self.moves.len() as u64 + 1;
        let id = format!("{kind}-{seq}");
        let mut m = Move {
            seq,
            id: id.clone(),
            kind: kind.into(),
            by: who.key.clone(),
            author: who.name.clone(),
            agent_of: who.agent_of_name.clone(),
            at,
            title: title.into(),
            body: body.into(),
            parent: parent.map(str::to_owned),
            fields,
            charter: self.charter_fingerprint().to_owned(),
            verb: verb.into(),
            args: args.clone(),
            seal: seal.clone(),
            prev: self.last_hash(),
            hash: String::new(),
            room_sig: String::new(),
        };
        m.hash = m.content_hash();
        m.room_sig = match replaying {
            // A kept move: the room's countersign must be over exactly what the replay made.
            Some(kept) => {
                if !countersigned(&self.args.room_key, &m.hash, &kept.room_sig) {
                    return Err(refused("the room didn't countersign this move as it stands"));
                }
                kept.room_sig.clone()
            }
            None => self.room_key.as_ref().ok_or_else(|| refused("this is a copy of the room; it can't make new moves"))?.countersign(&m.hash),
        };
        self.moves.push(m);
        Ok(id)
    }

    fn derive(&mut self, id: &str, state: Option<&str>, fields: &[(&str, Value)]) {
        let kind = self.moves.iter().find(|m| m.id == id).map(|m| m.kind.clone()).unwrap_or_default();
        let entry = self.derived.entry(id.to_owned()).or_insert_with(|| (first_state(&kind).to_owned(), Map::new()));
        if let Some(s) = state {
            entry.0 = s.to_owned();
        }
        for (k, v) in fields {
            entry.1.insert((*k).to_owned(), v.clone());
        }
    }

    fn state_of(&self, id: &str) -> Option<String> {
        let m = self.moves.iter().find(|m| m.id == id)?;
        Some(self.derived.get(id).map(|d| d.0.clone()).unwrap_or_else(|| first_state(&m.kind).to_owned()))
    }

    fn field_of(&self, id: &str, key: &str) -> Option<Value> {
        self.derived.get(id).and_then(|d| d.1.get(key).cloned()).or_else(|| self.moves.iter().find(|m| m.id == id).and_then(|m| m.fields.get(key).cloned()))
    }

    fn find(&self, id: &str, kind: &str) -> Result<Move, ErrorData> {
        self.moves.iter().find(|m| m.id == id && m.kind == kind).cloned().ok_or_else(|| bad(format!("no {kind} called {id}")))
    }

    /// Whether a receipt is one this room's host sealed, for this task and this doer.
    fn receipt_holds(&self, s: &Statement, task: &str, to: &str) -> bool {
        s.kind == "receipt" && s.holds() && s.key == self.args.host_key && s.field("room") == Some(self.args.id.as_str()) && s.field("task") == Some(task) && s.field("to") == Some(to)
    }

    /// One verb, applied. Everything a move records comes from `args` and the
    /// room's state, so a record can be replayed and checked.
    #[allow(clippy::too_many_arguments)]
    fn apply(&mut self, name: &str, args: &JsonObject, seal: &Seal, who: &Member, at: DateTime<Utc>, host: &dyn Host, replaying: Option<&Move>) -> Result<String, ErrorData> {
        let mut fields = Map::new();
        let pick = |fields: &mut Map<String, Value>, keys: &[&str]| {
            for k in keys {
                if let Some(v) = args.get(*k).filter(|v| !v.is_null() && v.as_str() != Some("")) {
                    fields.insert((*k).into(), v.clone());
                }
            }
        };
        let push = |room: &mut Room, kind: &str, title: &str, body: &str, parent: Option<&str>, fields: Map<String, Value>| room.push(at, who, name, args, seal, kind, title, body, parent, fields, replaying);
        Ok(match name {
            "show" => {
                let title = need(args, "title")?;
                let id = push(self, "show", title, arg(args, "body").unwrap_or_default(), None, fields)?;
                format!("Shown: {title} ({id})")
            }
            "ask" => {
                let what = need(args, "what")?;
                pick(&mut fields, &["needs", "ceiling", "who_may_serve", "thread"]);
                let id = push(self, "ask", what, "", None, fields)?;
                format!("Asked: {what} ({id})")
            }
            "offer" => {
                let ask = need(args, "ask_id")?.to_owned();
                let a = self.find(&ask, "ask")?;
                let state = self.state_of(&ask).unwrap_or_default();
                if state != "open" {
                    return Err(refused(format!("{ask} is {state}")));
                }
                let body = need(args, "body")?;
                let id = push(self, "offer", &a.title, body, Some(&ask), fields)?;
                let n = self.field_of(&ask, "offers").and_then(|v| v.as_u64()).unwrap_or(0) + 1;
                self.derive(&ask, None, &[("offers", json!(n))]);
                format!("Offered to serve {ask} ({id})")
            }
            "reply" => {
                let parent = need(args, "move_id")?.to_owned();
                if !self.moves.iter().any(|m| m.id == parent) {
                    return Err(bad(format!("no move called {parent}")));
                }
                let id = push(self, "reply", "", need(args, "body")?, Some(&parent), fields)?;
                format!("Replied to {parent} ({id})")
            }
            "say" => {
                let id = push(self, "say", "", need(args, "body")?, None, fields)?;
                format!("Said ({id})")
            }
            "report" => {
                let title = need(args, "title")?;
                pick(&mut fields, &["measured"]);
                let id = push(self, "run", title, arg(args, "body").unwrap_or_default(), None, fields)?;
                format!("Reported: {title} ({id})")
            }
            "post_offering" => {
                let title = need(args, "title")?;
                fields.insert("pricing".into(), json!(need(args, "pricing")?));
                pick(&mut fields, &["terms"]);
                let id = push(self, "offering", title, need(args, "what")?, None, fields)?;
                format!("Offered: {title} ({id})")
            }
            "post_task" => {
                let title = need(args, "title")?;
                pick(&mut fields, &["pledge"]);
                let id = push(self, "task", title, need(args, "spec")?, None, fields)?;
                format!("Task posted: {title} ({id})")
            }
            "claim" => {
                let task = need(args, "task_id")?.to_owned();
                let t = self.find(&task, "task")?;
                let state = self.state_of(&task).unwrap_or_default();
                if state != "open" {
                    return Err(refused(format!("{task} is {state}")));
                }
                push(self, "claim", &t.title, "", Some(&task), fields)?;
                self.derive(&task, Some("claimed"), &[("claimed_by", json!(who.name)), ("claimed_by_key", json!(who.key))]);
                format!("Claimed: {}", t.title)
            }
            "deliver" => {
                let task = need(args, "task_id")?.to_owned();
                let t = self.find(&task, "task")?;
                if self.field_of(&task, "claimed_by_key").and_then(|v| v.as_str().map(str::to_owned)).as_deref() != Some(who.key.as_str()) {
                    return Err(refused(format!("{} did not claim {task}", who.name)));
                }
                // A delivery can be redone until it's accepted, never after.
                let state = self.state_of(&task).unwrap_or_default();
                if state != "claimed" && state != "delivered" {
                    return Err(refused(format!("{task} is {state}")));
                }
                pick(&mut fields, &["files"]);
                push(self, "delivery", &t.title, need(args, "summary")?, Some(&task), fields)?;
                self.derive(&task, Some("delivered"), &[]);
                format!("Delivered: {}", t.title)
            }
            "accept" => {
                let task = need(args, "task_id")?.to_owned();
                let t = self.find(&task, "task")?;
                if t.by != who.key {
                    return Err(refused("only who posted a task may accept its delivery"));
                }
                let state = self.state_of(&task).unwrap_or_default();
                if state != "delivered" {
                    return Err(refused(format!("{task} is {state}")));
                }
                let to_key = self.field_of(&task, "claimed_by_key").and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default();
                let to = self.members.get(&to_key).cloned();
                let to_name = to.as_ref().map(|m| m.name.clone()).unwrap_or_default();
                let to_person = to.as_ref().and_then(|m| m.agent_of.clone()).unwrap_or_else(|| to_key.clone());
                // On a replay the receipt is the one the record holds; nothing is sealed again.
                let statement: Statement = match replaying {
                    Some(m) => m.fields.get("statement").and_then(|v| serde_json::from_value(v.clone()).ok()).ok_or_else(|| refused("the record's receipt is missing"))?,
                    None => host
                        .seal(
                            "receipt",
                            json!({ "room": self.args.id, "room_title": self.args.title, "host": self.args.host_name, "task": task, "title": t.title, "to": to_key, "to_name": to_name, "to_person": to_person, "issued": at }),
                        )
                        .map_err(refused)?,
                };
                if !self.receipt_holds(&statement, &task, &to_key) {
                    return Err(refused("that receipt isn't sealed by this room's host, for this task"));
                }
                fields.insert("to".into(), json!(to_name));
                fields.insert("statement".into(), serde_json::to_value(&statement).unwrap_or_default());
                push(self, "receipt", &t.title, &format!("Completed: {}", t.title), Some(&task), fields)?;
                self.derive(&task, Some("done"), &[]);
                format!("Accepted: {}", t.title)
            }
            "settle" => {
                let task = need(args, "task_id")?.to_owned();
                let t = self.find(&task, "task")?;
                let agree = args.get("agree").and_then(Value::as_bool).ok_or_else(|| bad("agree is needed"))?;
                if self.state_of(&task).as_deref() != Some("done") {
                    return Err(refused(format!("{task} isn't done, so there's nothing to settle yet")));
                }
                let claimer = self.field_of(&task, "claimed_by_key").and_then(|v| v.as_str().map(str::to_owned));
                let side = if t.by == who.key {
                    "poster_says"
                } else if claimer.as_deref() == Some(who.key.as_str()) || self.members.get(claimer.as_deref().unwrap_or_default()).and_then(|m| m.agent_of.clone()).as_deref() == Some(who.key.as_str()) {
                    "doer_says"
                } else {
                    return Err(refused("only whoever posted it or whoever did it may say it's settled"));
                };
                if self.field_of(&task, side).is_some() {
                    return Err(refused("you've already said whether it's settled"));
                }
                pick(&mut fields, &["note"]);
                fields.insert("agree".into(), json!(agree));
                push(self, "settle", &t.title, arg(args, "note").unwrap_or_default(), Some(&task), fields)?;
                self.derive(&task, None, &[(side, json!({ "agree": agree, "note": arg(args, "note") }))]);
                format!("Said {}: {}", if agree { "settled" } else { "not settled" }, t.title)
            }
            "propose" => {
                let direction = need(args, "direction")?;
                let id = push(self, "direction", direction, arg(args, "body").unwrap_or_default(), None, fields)?;
                format!("Proposed: {direction} ({id})")
            }
            "steer" => {
                let target = need(args, "direction_id")?.to_owned();
                let d = self.find(&target, "direction")?;
                let mv = need(args, "move")?.to_owned();
                if !["prefer", "reject", "note"].contains(&mv.as_str()) {
                    return Err(bad("a steer is prefer, reject or note"));
                }
                fields.insert("move".into(), json!(mv));
                push(self, "steer", &d.title, arg(args, "note").unwrap_or_default(), Some(&target), fields)?;
                let n = self.field_of(&target, &mv).and_then(|v| v.as_u64()).unwrap_or(0) + 1;
                self.derive(&target, None, &[(mv.as_str(), json!(n))]);
                format!("Steered {}: {mv}", d.title)
            }
            "synthesize" => {
                let id = push(self, "synthesis", "Where it stands", need(args, "body")?, None, fields)?;
                format!("Synthesis written ({id})")
            }
            "pin_receipt" => {
                let statement: Statement = serde_json::from_value(args.get("statement").cloned().unwrap_or_default()).map_err(|_| bad("that is not a receipt"))?;
                if statement.kind != "receipt" || !statement.holds() {
                    return Err(refused("that receipt's seal does not hold"));
                }
                if statement.field("to_person") != Some(self.args.host_key.as_str()) {
                    return Err(refused("that receipt was issued to someone else"));
                }
                // A receipt proves its room only if it was sealed by the host that room's id names.
                if statement.key == self.args.host_key || !statement.field("room").is_some_and(|room| id_holds(room, &statement.key)) {
                    return Err(refused("that receipt wasn't sealed by the host of the room it names"));
                }
                let title = statement.field("title").unwrap_or_default().to_owned();
                fields.insert("statement".into(), serde_json::to_value(&statement).unwrap_or_default());
                let id = push(self, "pinned_receipt", &title, &format!("Issued by {}", statement.field("room_title").unwrap_or_default()), None, fields)?;
                format!("Pinned: {title} ({id})")
            }
            "leave_note" => {
                let id = push(self, "note", "", need(args, "body")?, None, fields)?;
                format!("Left a note ({id})")
            }
            "hire" => {
                if who.key == self.args.host_key {
                    return Err(refused("you can't hire your own agents here; message them"));
                }
                let agent = need(args, "agent")?.to_owned();
                let what = need(args, "what")?.to_owned();
                pick(&mut fields, &["agent", "pledge"]);
                let id = push(self, "hire", &what, "", None, fields)?;
                if replaying.is_none() {
                    host.hire(&self.args.id, &id, &who.name, &agent, &what, arg(args, "pledge"));
                }
                format!("Asked {} for: {what} ({id})", self.args.host_name)
            }
            "answer_hire" => {
                let hire = need(args, "hire_id")?.to_owned();
                let h = self.find(&hire, "hire")?;
                let state = self.state_of(&hire).unwrap_or_default();
                if state != "asked" {
                    return Err(refused(format!("{hire} was already {state}")));
                }
                let take = args.get("take").and_then(Value::as_bool).ok_or_else(|| bad("take is needed"))?;
                pick(&mut fields, &["note"]);
                fields.insert("take".into(), json!(take));
                push(self, "hire_answer", &h.title, arg(args, "note").unwrap_or_default(), Some(&hire), fields)?;
                self.derive(&hire, Some(if take { "taken" } else { "declined" }), &[]);
                format!("{}: {}", if take { "Taken" } else { "Declined" }, h.title)
            }
            "deliver_hire" => {
                let hire = need(args, "hire_id")?.to_owned();
                let h = self.find(&hire, "hire")?;
                if self.state_of(&hire).as_deref() != Some("taken") {
                    return Err(refused(format!("{hire} isn't taken")));
                }
                pick(&mut fields, &["files"]);
                push(self, "hire_delivery", &h.title, need(args, "summary")?, Some(&hire), fields)?;
                self.derive(&hire, Some("delivered"), &[]);
                format!("Delivered: {}", h.title)
            }
            "admit" => {
                let key = need(args, "key")?.to_owned();
                let member_name = need(args, "name")?.to_owned();
                let is_agent = args.get("is_agent").and_then(Value::as_bool).unwrap_or(false);
                let listed = args.get("listed").and_then(Value::as_bool).unwrap_or(true);
                let (agent_of, agent_of_name) = if is_agent {
                    let person = need(args, "agent_of")?.to_owned();
                    let tether: Statement = serde_json::from_value(args.get("tether").cloned().unwrap_or_default()).map_err(|_| bad("an agent needs its person's tether"))?;
                    if tether.kind != "tether" || !tether.holds() || tether.key != person || tether.field("agent") != Some(key.as_str()) {
                        return Err(refused("that agent's tether does not hold"));
                    }
                    let owner = self.members.get(&person).filter(|m| !m.removed).ok_or_else(|| refused("an agent's person must be a member first"))?;
                    (Some(person), Some(owner.name.clone()))
                } else {
                    (None, None)
                };
                if self.members.get(&key).is_some_and(|m| !m.removed) {
                    return Err(refused(format!("{member_name} is already a member")));
                }
                fields.insert("key".into(), json!(key));
                push(self, "admitted", &member_name, "", None, fields)?;
                self.members.insert(key.clone(), Member { key, name: member_name.clone(), is_agent, agent_of, agent_of_name, listed, joined: at, removed: false });
                self.notify(MEMBERS);
                format!("Admitted: {member_name}")
            }
            "remove" => {
                let key = need(args, "key")?.to_owned();
                if key == self.args.host_key {
                    return Err(refused("the host can't remove themselves; end the room instead"));
                }
                let gone = self.members.get(&key).filter(|m| !m.removed).map(|m| m.name.clone()).ok_or_else(|| bad("no such member"))?;
                // An agent goes with its person, in the same move.
                let agents: Vec<Key> = self.members.values().filter(|o| !o.removed && o.agent_of.as_deref() == Some(key.as_str())).map(|o| o.key.clone()).collect();
                pick(&mut fields, &["reason"]);
                fields.insert("key".into(), json!(key));
                if !agents.is_empty() {
                    fields.insert("also".into(), json!(agents));
                }
                push(self, "removed", &gone, arg(args, "reason").unwrap_or_default(), None, fields)?;
                for k in agents.iter().chain(std::iter::once(&key)) {
                    if let Some(m) = self.members.get_mut(k) {
                        m.removed = true;
                    }
                }
                self.notify(MEMBERS);
                format!("Removed: {gone}")
            }
            "set_charter" => {
                let text = need(args, "text")?.to_owned();
                let fp = fingerprint(&text);
                fields.insert("fingerprint".into(), json!(fp));
                push(self, "charter", "The rules changed", "", None, fields)?;
                self.charters.push(Charter { fingerprint: fp, text, at });
                self.notify(CHARTER);
                "Rules changed".to_owned()
            }
            "vouch_room" => {
                let title = need(args, "title")?;
                let id = push(self, "doorway", title, need(args, "invite")?, None, fields)?;
                self.notify(DOORWAYS);
                format!("Vouched for {title} ({id})")
            }
            other => return Err(bad(format!("this room has no verb called {other}"))),
        })
    }
}
