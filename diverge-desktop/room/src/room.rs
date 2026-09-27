//! One room: what a Space's tool container runs.
//!
//! Its verbs are MCP tools, its objects are *moves*, and every change is an
//! MCP resource-updated notification, which the wire fans out to every
//! member. What makes it a room people can trust:
//!
//! - **Every call is sealed** (see [`crate::seal`]). The room checks the
//!   seal against the keys its host admitted, and refuses anything else.
//! - **Every move is chained**: it carries the hash of the move before it,
//!   and the verb, arguments and seal that made it, so anyone holding a
//!   copy can check the whole record without trusting whoever served it.
//! - **Moves never change.** What changes (a task claimed, a hire taken) is
//!   derived from later moves.
//! - **Receipts are sealed by the host**, so a receipt proves which room
//!   issued it wherever it is shown.
//!
//! Admissions, removals and charter changes are moves too, so a room can be
//! rebuilt from its record alone: restarted by its host, or continued by a
//! member when the host is gone.

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

use crate::seal::{Key, Seal, Statement, canonical, check_call, digest, fingerprint, recheck};

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

/// What a host starts a room with: the container's `arguments`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Args {
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Continues {
    pub room: String,
    pub title: String,
    /// The hash of the last move of the room it continues.
    pub last: String,
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
}

impl Move {
    fn content_hash(&self) -> String {
        let mut v = serde_json::to_value(self).unwrap_or_default();
        if let Some(o) = v.as_object_mut() {
            o.remove("hash");
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

#[derive(Clone)]
struct Charter {
    fingerprint: String,
    text: String,
    at: DateTime<Utc>,
}

pub struct Room {
    pub args: Args,
    charters: Vec<Charter>,
    members: IndexMap<Key, Member>,
    moves: Vec<Move>,
    /// What later moves made of earlier ones: id → (state, fields).
    derived: HashMap<String, (String, Map<String, Value>)>,
    counters: HashMap<Key, u64>,
    /// The record of the room this one continues, checked, shown first.
    history: Vec<Move>,
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
    pub fn new(args: Args) -> Self {
        Self::with_history(args, Vec::new(), DateTime::<Utc>::MIN_UTC)
    }

    fn with_history(args: Args, history: Vec<Move>, at: DateTime<Utc>) -> Self {
        let (live, _) = broadcast::channel(256);
        let charter = Charter { fingerprint: fingerprint(&args.charter), text: args.charter.clone(), at };
        let mut members = IndexMap::new();
        members.insert(
            args.host_key.clone(),
            Member { key: args.host_key.clone(), name: args.host_name.clone(), is_agent: false, agent_of: None, agent_of_name: None, listed: true, joined: at, removed: false },
        );
        Room { args, charters: vec![charter], members, moves: Vec::new(), derived: HashMap::new(), counters: HashMap::new(), history, live }
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
                let from = self.args.continues.as_ref().map(|c| c.title.as_str());
                // Someone who asked not to be listed isn't announced; the record still holds it.
                let shown = |m: &&Move| !(m.kind == "admitted" && m.args.get("listed").and_then(Value::as_bool) == Some(false));
                let all: Vec<Value> = self.history.iter().filter(shown).map(|m| self.served(m, from)).chain(self.moves.iter().filter(shown).map(|m| self.served(m, None))).collect();
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
                })
                .to_string(),
                "application/json",
            ),
            RECORD => (json!({ "args": self.args, "history": self.history, "moves": self.moves }).to_string(), "application/json"),
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
        let host_verb = self.has_verb(&name).ok_or_else(|| bad(format!("this room has no verb called {name}")))?;
        let seal = check_call(&self.args.id, &params).map_err(refused)?;
        if seal.counter <= self.counters.get(&seal.key).copied().unwrap_or(0) {
            return Err(refused("this call was already made"));
        }
        let is_host = seal.key == self.args.host_key;
        let who = match self.members.get(&seal.key) {
            Some(m) if m.removed => return Err(refused(format!("{} was removed from this room", m.name))),
            Some(m) => m.clone(),
            None => return Err(refused("that key is not a member here")),
        };
        if host_verb && !is_host {
            return Err(refused("only the host may do that"));
        }
        let args = params.arguments.clone().unwrap_or_default();
        let line = self.apply(&name, &args, &seal, &who, at, host, None)?;
        self.counters.insert(seal.key.clone(), seal.counter);
        self.notify(FEED);
        Ok(CallToolResult::success(vec![ContentBlock::text(line)]))
    }

    #[allow(clippy::too_many_arguments)]
    fn push(&mut self, at: DateTime<Utc>, who: &Member, verb: &str, args: &JsonObject, seal: &Seal, kind: &str, title: &str, body: &str, parent: Option<&str>, fields: Map<String, Value>) -> String {
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
        };
        m.hash = m.content_hash();
        self.moves.push(m);
        id
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

    /// One verb, applied. Everything a move records comes from `args`, so a
    /// copy of the record can be replayed and checked.
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
        let push = |room: &mut Room, kind: &str, title: &str, body: &str, parent: Option<&str>, fields: Map<String, Value>| room.push(at, who, name, args, seal, kind, title, body, parent, fields);
        Ok(match name {
            "show" => {
                let title = need(args, "title")?;
                let id = push(self, "show", title, arg(args, "body").unwrap_or_default(), None, fields);
                format!("Shown: {title} ({id})")
            }
            "ask" => {
                let what = need(args, "what")?;
                pick(&mut fields, &["needs", "ceiling", "who_may_serve", "thread"]);
                let id = push(self, "ask", what, "", None, fields);
                format!("Asked: {what} ({id})")
            }
            "offer" => {
                let ask = need(args, "ask_id")?.to_owned();
                let a = self.find(&ask, "ask")?;
                let body = need(args, "body")?;
                let id = push(self, "offer", &a.title, body, Some(&ask), fields);
                let n = self.field_of(&ask, "offers").and_then(|v| v.as_u64()).unwrap_or(0) + 1;
                self.derive(&ask, None, &[("offers", json!(n))]);
                format!("Offered to serve {ask} ({id})")
            }
            "reply" => {
                let parent = need(args, "move_id")?.to_owned();
                if !self.moves.iter().any(|m| m.id == parent) {
                    return Err(bad(format!("no move called {parent}")));
                }
                let id = push(self, "reply", "", need(args, "body")?, Some(&parent), fields);
                format!("Replied to {parent} ({id})")
            }
            "say" => {
                let id = push(self, "say", "", need(args, "body")?, None, fields);
                format!("Said ({id})")
            }
            "report" => {
                let title = need(args, "title")?;
                pick(&mut fields, &["measured"]);
                let id = push(self, "run", title, arg(args, "body").unwrap_or_default(), None, fields);
                format!("Reported: {title} ({id})")
            }
            "post_offering" => {
                let title = need(args, "title")?;
                fields.insert("pricing".into(), json!(need(args, "pricing")?));
                pick(&mut fields, &["terms"]);
                let id = push(self, "offering", title, need(args, "what")?, None, fields);
                format!("Offered: {title} ({id})")
            }
            "post_task" => {
                let title = need(args, "title")?;
                pick(&mut fields, &["pledge"]);
                let id = push(self, "task", title, need(args, "spec")?, None, fields);
                format!("Task posted: {title} ({id})")
            }
            "claim" => {
                let task = need(args, "task_id")?.to_owned();
                let t = self.find(&task, "task")?;
                let state = self.state_of(&task).unwrap_or_default();
                if state != "open" {
                    return Err(refused(format!("{task} is {state}")));
                }
                push(self, "claim", &t.title, "", Some(&task), fields);
                self.derive(&task, Some("claimed"), &[("claimed_by", json!(who.name)), ("claimed_by_key", json!(who.key))]);
                format!("Claimed: {}", t.title)
            }
            "deliver" => {
                let task = need(args, "task_id")?.to_owned();
                let t = self.find(&task, "task")?;
                if self.field_of(&task, "claimed_by_key").and_then(|v| v.as_str().map(str::to_owned)).as_deref() != Some(who.key.as_str()) {
                    return Err(refused(format!("{} did not claim {task}", who.name)));
                }
                pick(&mut fields, &["files"]);
                push(self, "delivery", &t.title, need(args, "summary")?, Some(&task), fields);
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
                let kept: Option<Statement> = replaying.and_then(|m| m.fields.get("statement")).and_then(|v| serde_json::from_value(v.clone()).ok());
                let statement = match kept {
                    Some(s) => s,
                    None => host
                        .seal(
                            "receipt",
                            json!({ "room": self.args.id, "room_title": self.args.title, "host": self.args.host_name, "task": task, "title": t.title, "to": to_key, "to_name": to_name, "to_person": to_person, "issued": at }),
                        )
                        .map_err(refused)?,
                };
                fields.insert("to".into(), json!(to_name));
                fields.insert("statement".into(), serde_json::to_value(&statement).unwrap_or_default());
                push(self, "receipt", &t.title, &format!("Completed: {}", t.title), Some(&task), fields);
                self.derive(&task, Some("done"), &[]);
                format!("Accepted: {}", t.title)
            }
            "settle" => {
                let task = need(args, "task_id")?.to_owned();
                let t = self.find(&task, "task")?;
                let agree = args.get("agree").and_then(Value::as_bool).ok_or_else(|| bad("agree is needed"))?;
                let claimer = self.field_of(&task, "claimed_by_key").and_then(|v| v.as_str().map(str::to_owned));
                let side = if t.by == who.key {
                    "poster_says"
                } else if claimer.as_deref() == Some(who.key.as_str()) || self.members.get(claimer.as_deref().unwrap_or_default()).and_then(|m| m.agent_of.clone()).as_deref() == Some(who.key.as_str()) {
                    "doer_says"
                } else {
                    return Err(refused("only whoever posted it or whoever did it may say it's settled"));
                };
                pick(&mut fields, &["note"]);
                fields.insert("agree".into(), json!(agree));
                push(self, "settle", &t.title, arg(args, "note").unwrap_or_default(), Some(&task), fields);
                self.derive(&task, None, &[(side, json!({ "agree": agree, "note": arg(args, "note") }))]);
                format!("Said {}: {}", if agree { "settled" } else { "not settled" }, t.title)
            }
            "propose" => {
                let direction = need(args, "direction")?;
                let id = push(self, "direction", direction, arg(args, "body").unwrap_or_default(), None, fields);
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
                push(self, "steer", &d.title, arg(args, "note").unwrap_or_default(), Some(&target), fields);
                let n = self.field_of(&target, &mv).and_then(|v| v.as_u64()).unwrap_or(0) + 1;
                self.derive(&target, None, &[(mv.as_str(), json!(n))]);
                format!("Steered {}: {mv}", d.title)
            }
            "synthesize" => {
                let id = push(self, "synthesis", "Where it stands", need(args, "body")?, None, fields);
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
                let title = statement.field("title").unwrap_or_default().to_owned();
                fields.insert("statement".into(), serde_json::to_value(&statement).unwrap_or_default());
                let id = push(self, "pinned_receipt", &title, &format!("Issued by {}", statement.field("room_title").unwrap_or_default()), None, fields);
                format!("Pinned: {title} ({id})")
            }
            "leave_note" => {
                let id = push(self, "note", "", need(args, "body")?, None, fields);
                format!("Left a note ({id})")
            }
            "hire" => {
                if who.key == self.args.host_key {
                    return Err(refused("you can't hire your own agents here; message them"));
                }
                let agent = need(args, "agent")?.to_owned();
                let what = need(args, "what")?.to_owned();
                pick(&mut fields, &["agent", "pledge"]);
                let id = push(self, "hire", &what, "", None, fields);
                if replaying.is_none() {
                    host.hire(&self.args.id, &id, &who.name, &agent, &what, arg(args, "pledge"));
                }
                format!("Asked {} for: {what} ({id})", self.args.host_name)
            }
            "answer_hire" => {
                let hire = need(args, "hire_id")?.to_owned();
                let h = self.find(&hire, "hire")?;
                let take = args.get("take").and_then(Value::as_bool).ok_or_else(|| bad("take is needed"))?;
                pick(&mut fields, &["note"]);
                fields.insert("take".into(), json!(take));
                push(self, "hire_answer", &h.title, arg(args, "note").unwrap_or_default(), Some(&hire), fields);
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
                push(self, "hire_delivery", &h.title, need(args, "summary")?, Some(&hire), fields);
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
                push(self, "admitted", &member_name, "", None, fields);
                self.members.insert(key.clone(), Member { key, name: member_name.clone(), is_agent, agent_of, agent_of_name, listed, joined: at, removed: false });
                self.notify(MEMBERS);
                format!("Admitted: {member_name}")
            }
            "remove" => {
                let key = need(args, "key")?.to_owned();
                if key == self.args.host_key {
                    return Err(refused("the host can't remove themselves; end the room instead"));
                }
                let m = self.members.get_mut(&key).filter(|m| !m.removed).ok_or_else(|| bad("no such member"))?;
                m.removed = true;
                let gone = m.name.clone();
                // An agent goes with its person.
                for other in self.members.values_mut() {
                    if other.agent_of.as_deref() == Some(key.as_str()) {
                        other.removed = true;
                    }
                }
                pick(&mut fields, &["reason"]);
                fields.insert("key".into(), json!(key));
                push(self, "removed", &gone, arg(args, "reason").unwrap_or_default(), None, fields);
                self.notify(MEMBERS);
                format!("Removed: {gone}")
            }
            "set_charter" => {
                let text = need(args, "text")?.to_owned();
                let fp = fingerprint(&text);
                self.charters.push(Charter { fingerprint: fp.clone(), text, at });
                fields.insert("fingerprint".into(), json!(fp));
                push(self, "charter", "The rules changed", "", None, fields);
                self.notify(CHARTER);
                "Rules changed".to_owned()
            }
            "vouch_room" => {
                let title = need(args, "title")?;
                let id = push(self, "doorway", title, need(args, "invite")?, None, fields);
                self.notify(DOORWAYS);
                format!("Vouched for {title} ({id})")
            }
            other => return Err(bad(format!("this room has no verb called {other}"))),
        })
    }

    /// Rebuild a room from its record: check every link and every seal,
    /// then replay. A restart by its host, or a successor started by a member.
    pub fn from_record(args: Args, history: Vec<Move>, moves: &[Move], host: &dyn Host) -> Result<Room, String> {
        check_record(&args.id, moves)?;
        if let Some(c) = &args.continues {
            check_record(&c.room, &history)?;
            if history.last().map(|m| m.hash.as_str()) != Some(c.last.as_str()) {
                return Err("that history doesn't end where the room it continues ended".into());
            }
        }
        let at = moves.first().map(|m| m.at).unwrap_or_else(Utc::now);
        let mut room = Room::with_history(args, history, at);
        for m in moves {
            let who = room.members.get(&m.seal.key).cloned().ok_or_else(|| format!("{} was made by someone never admitted", m.id))?;
            room.apply(&m.verb, &m.args, &m.seal, &who, m.at, host, Some(m)).map_err(|e| format!("{} does not replay: {}", m.id, e.message))?;
            room.counters.insert(m.seal.key.clone(), m.seal.counter);
            if room.moves.last() != Some(m) {
                return Err(format!("{} replays differently", m.id));
            }
        }
        Ok(room)
    }
}

/// Every move links to the one before it, hashes to what it says, and was
/// sealed by the key it names, for the room it names.
pub fn check_record(room: &str, moves: &[Move]) -> Result<(), String> {
    let mut prev = String::new();
    for m in moves {
        if m.prev != prev {
            return Err(format!("{} does not follow the move before it", m.id));
        }
        if m.content_hash() != m.hash {
            return Err(format!("{} was changed after it was made", m.id));
        }
        if m.seal.key != m.by || !recheck(room, &m.verb, &m.args, &m.seal) {
            return Err(format!("{}'s seal does not hold", m.id));
        }
        prev = m.hash.clone();
    }
    Ok(())
}
