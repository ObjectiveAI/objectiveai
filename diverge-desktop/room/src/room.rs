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
//! - **Someone let in unlisted is a mark in the record**, not a key, until
//!   they first act: the mark is of their key and this room, so it can't be
//!   matched against the same key elsewhere. Their moves name their key, as
//!   everyone's do.
//!
//! - **Rules have versions.** A room's settings say which rules it runs
//!   under, and a replay picks them by that number. Under rules 1 a member
//!   is a key. Under rules 2 a person is their account (see
//!   [`crate::account`]): any key on its newest device list acts for them, a
//!   `keys` move brings a newer list, and the room's id names the host's
//!   account, so the host can act from any of its devices.
//!
//! One thing no record can show is what came after it: a copy that stops
//! early is a true copy of an earlier moment. Copies say when they end.

use std::collections::{HashMap, HashSet};
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

use crate::account::Proof;
use crate::seal::{Key, Keypair, Seal, Statement, account_id_holds, canonical, check_call, countersigned, digest, fingerprint, id_holds, recheck, statement_holds};

pub const FEED: &str = "space://feed";
pub const CHARTER: &str = "space://charter";
pub const CHARTERS: &str = "space://charters";
pub const MEMBERS: &str = "space://members";
pub const DOORWAYS: &str = "space://doorways";
pub const ABOUT: &str = "space://about";
pub const RECORD: &str = "space://record";

/// The `_meta` key that marks a verb only the host may use.
pub const META_HOST_ONLY: &str = "network.diverge.desktop/host_only";

/// What a call from a key the room doesn't know gets. Under rules 2 it may
/// be a member's device the room hasn't been shown: their `keys` move first.
pub const NOT_A_MEMBER: &str = "that key is not a member here";

/// The rules a room runs under, by the number its settings carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rules {
    /// The first rules: a member is a key.
    One,
    /// A person is their account: any key on its newest device list acts for them.
    Two,
}

impl Rules {
    pub fn of(number: u32) -> Result<Rules, String> {
        match number {
            1 => Ok(Rules::One),
            2 => Ok(Rules::Two),
            n => Err(format!("this room runs under rules number {n}, and this program knows only rules 1 and 2, so it won't run or check it")),
        }
    }
}

fn rules_one() -> u32 {
    1
}

fn is_rules_one(number: &u32) -> bool {
    *number == 1
}

/// Where someone stands in a room, as its record has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    /// Let in, and on the list or acted since; not removed.
    Member,
    /// Let in unlisted and never acted: the record knows them by this mark only.
    Unlisted { mark: String },
    /// Let in once, then removed.
    Removed,
    /// Never let in.
    Stranger,
}

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
    /// The rules the room runs under. Rules 1 aren't written down, so
    /// settings from before rules had numbers read and sign as they did.
    #[serde(default = "rules_one", skip_serializing_if = "is_rules_one")]
    pub rules: u32,
    /// Under rules 2: the host's account, which names `host_key` as one of
    /// its devices. The room's id names this account, not the key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_account: Option<Proof>,
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

    /// Whether these are settings their host made, under rules this program
    /// knows: the id is theirs and they signed it all. Under rules 2 the id
    /// is their account's, and the key that signed is one of its devices.
    pub fn holds(&self) -> Result<(), String> {
        match Rules::of(self.rules)? {
            Rules::One => {
                if self.host_account.is_some() {
                    return Err("settings under rules 1 name no account".into());
                }
                if !id_holds(&self.id, &self.host_key) {
                    return Err(format!("{} isn't an id its host could have made", self.id));
                }
            }
            Rules::Two => {
                let account = self.host_account.as_ref().ok_or("settings under rules 2 name their host's account")?.check()?;
                if !account_id_holds(&self.id, &account.id) {
                    return Err(format!("{} isn't an id its host's account could have made", self.id));
                }
                if !account.devices.contains(&self.host_key) {
                    return Err("the key that signed these settings isn't on its host's device list".into());
                }
            }
        }
        if !statement_holds(&self.host_key, "room", &self.body(), &self.sig) {
            return Err("this room's settings aren't signed by its host".into());
        }
        Ok(())
    }

    /// Who hosts the room, as its members are keyed: the host's key under
    /// rules 1, their account under rules 2.
    pub fn host_id(&self) -> String {
        match &self.host_account {
            Some(account) if self.rules == 2 => account.id(),
            _ => self.host_key.clone(),
        }
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
    /// Under rules 2, a person's account: its genesis and the newest device
    /// list the room has been shown. `key` is then the account's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<Proof>,
}

/// One move, as it was made. Never changed afterwards.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Move {
    pub seq: u64,
    pub id: String,
    pub kind: String,
    /// The key that made it.
    pub by: Key,
    /// Under rules 2, the member it acts for when that isn't the key: a
    /// person's account, acting from one of its devices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<String>,
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
    /// The member who made it: their account under rules 2, their key otherwise.
    pub fn actor(&self) -> &str {
        self.member.as_deref().unwrap_or(&self.by)
    }

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

/// The mark a record keeps for someone let in unlisted: of their key and
/// this room, so it can't be matched against the same key in another room.
pub fn key_mark(room: &str, key: &str) -> String {
    digest(format!("diverge-desktop member\n{room}\n{key}").as_bytes())[..32].to_owned()
}

/// Who sealed a receipt, if the room it names is theirs: the sealing key,
/// for a room under rules 1; for a room under rules 2, the account the
/// receipt carries, which must name that key and be the one the id names.
pub fn receipt_issuer(s: &Statement) -> Option<String> {
    let room = s.field("room")?;
    match s.body.get("host_account") {
        None => id_holds(room, &s.key).then(|| s.key.clone()),
        Some(v) => {
            let account = serde_json::from_value::<Proof>(v.clone()).ok()?.check().ok()?;
            (account_id_holds(room, &account.id) && account.devices.contains(&s.key)).then_some(account.id)
        }
    }
}

fn proof_arg(args: &JsonObject) -> Result<Proof, ErrorData> {
    serde_json::from_value(args.get("account").cloned().unwrap_or_default()).map_err(|_| bad("account needs an account's genesis and device list"))
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
    rules: Rules,
    /// The room's own key, when this is the room itself; `None` for a copy
    /// someone is checking, which can't make new moves.
    room_key: Option<Keypair>,
    charters: Vec<Charter>,
    members: IndexMap<Key, Member>,
    moves: Vec<Move>,
    /// What later moves made of earlier ones: id → (state, fields).
    derived: HashMap<String, (String, Map<String, Value>)>,
    counters: HashMap<Key, u64>,
    /// Let in unlisted, not yet acted: their key's mark → the name they gave, and when.
    unlisted: HashMap<String, (String, DateTime<Utc>)>,
    /// Marks of people let in unlisted and removed before they acted.
    gone: HashSet<String>,
    /// Under rules 2: each key on a person's current device list → their account.
    devices: HashMap<Key, String>,
    /// Under rules 2: keys a newer device list left off → the account they were on.
    retired: HashMap<Key, String>,
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
        let rules = Rules::of(args.rules)?;
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
                // Whoever starts it, as the old room knew them, by what they prove: the key that
                // signed these settings, or under rules 2 the account whose root named that key.
                // Other keys on that account's list are only the root's word, so they count for nothing here.
                let still_in = old.person_in(&args.host_key) || (rules == Rules::Two && old.person_in(&args.host_id()));
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
        let host = args.host_id();
        let account = if rules == Rules::Two { args.host_account.clone() } else { None };
        let mut devices = HashMap::new();
        for device in account.as_ref().and_then(|p| p.check().ok()).map(|a| a.devices).unwrap_or_default() {
            devices.insert(device, host.clone());
        }
        let mut members = IndexMap::new();
        members.insert(host.clone(), Member { key: host, name: args.host_name.clone(), is_agent: false, agent_of: None, agent_of_name: None, listed: true, joined: args.at, removed: false, account });
        Ok(Room {
            args,
            rules,
            room_key,
            charters: vec![charter],
            members,
            moves: Vec::new(),
            derived: HashMap::new(),
            counters: HashMap::new(),
            unlisted: HashMap::new(),
            gone: HashSet::new(),
            devices,
            retired: HashMap::new(),
            old,
            live,
        })
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

    pub fn rules(&self) -> Rules {
        self.rules
    }

    /// The member a key acts for: under rules 2, the account whose current
    /// device list names it; otherwise the key itself.
    pub fn member_id(&self, who: &str) -> String {
        self.devices.get(who).cloned().unwrap_or_else(|| who.to_owned())
    }

    /// Whether someone (a key, or under rules 2 an account) may still read
    /// the room: let in, not removed.
    pub fn may_read(&self, who: &str) -> bool {
        let id = self.member_id(who);
        self.members.get(&id).is_some_and(|m| !m.removed) || self.unlisted.contains_key(&key_mark(&self.args.id, &id))
    }

    /// Where someone stands here (a key, or under rules 2 an account or any
    /// key on its current list), as the record has it. Reads; changes nothing.
    pub fn standing(&self, who: &str) -> Standing {
        let id = self.member_id(who);
        let mark = key_mark(&self.args.id, &id);
        match self.members.get(&id) {
            Some(m) if m.removed => Standing::Removed,
            Some(_) => Standing::Member,
            None if self.unlisted.contains_key(&mark) => Standing::Unlisted { mark },
            None if self.gone.contains(&mark) => Standing::Removed,
            None => Standing::Stranger,
        }
    }

    /// Whether someone is a person still in the room (not an agent).
    fn person_in(&self, who: &str) -> bool {
        self.members.get(&self.member_id(who)).is_some_and(|m| !m.removed && !m.is_agent)
    }

    /// Whether a key is the host's: theirs under rules 1, on their account's current list under rules 2.
    fn host_holds(&self, key: &str) -> bool {
        self.member_id(key) == self.args.host_id()
    }

    /// A person's current devices: those before that the list leaves off retire.
    fn set_devices(&mut self, id: &str, devices: &[Key]) {
        let before: Vec<Key> = self.devices.iter().filter(|(_, m)| m.as_str() == id).map(|(k, _)| k.clone()).collect();
        for k in before {
            if !devices.contains(&k) {
                self.devices.remove(&k);
                self.retired.insert(k, id.to_owned());
            }
        }
        for d in devices {
            self.retired.remove(d);
            self.devices.insert(d.clone(), id.to_owned());
        }
    }

    /// The keys a person's list names, when one of them already acts for someone else here.
    fn taken(&self, id: &str, devices: &[Key]) -> bool {
        devices.iter().any(|d| self.devices.get(d).is_some_and(|m| m != id) || (self.members.contains_key(d) && d != id))
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
                tools.push(verb(
                    "take_offer",
                    "Take an offer on your ask. The ask is taken; on a work board the offer becomes a task whoever offered already holds, and ends in a receipt like any other.",
                    json!({ "offer_id": { "type": "string" } }),
                    &["offer_id"],
                ));
                tools.push(verb("close_ask", "Close your ask: nobody can offer on it any more.", json!({ "ask_id": { "type": "string" }, "note": { "type": "string" } }), &["ask_id"]));
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
        if self.rules == Rules::Two {
            tools.push(verb(
                "keys",
                "Bring your account's newest device list: the keys that act for you now. A key it leaves off can't act here any more.",
                json!({ "account": { "type": "object", "description": "Your account's genesis and its newest device list." } }),
                &["account"],
            ));
        }
        // The host's own verbs.
        tools.push(host_only(verb(
            "admit",
            "Let someone in: a person you said yes to at the door, or an agent tethered to a member.",
            json!({
                "key": { "type": "string", "description": "An agent's key; under rules 1, a person's too." },
                "account": { "type": "object", "description": "Under rules 2: the person's account, its genesis and newest device list." },
                "name": { "type": "string" },
                "is_agent": { "type": "boolean" },
                "agent_of": { "type": "string", "description": "For an agent: its person's key." },
                "tether": { "type": "object", "description": "For an agent: its person's statement that the key is theirs." },
                "listed": { "type": "boolean", "description": "Whether they asked to be on the room's list." }
            }),
            &["name"],
        )));
        tools.push(host_only(verb(
            "remove",
            "Remove someone: the room refuses their calls from now on.",
            json!({ "key": { "type": "string" }, "reason": { "type": "string" }, "quiet": { "type": "boolean", "description": "Leave it off the room's feed. The record keeps it." } }),
            &["key"],
        )));
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
                // Someone who asked not to be listed isn't announced, nor a removal the host kept quiet; the record holds both.
                // Nor a member bringing their device list: that's for the room to check, not to announce.
                let shown = |m: &&Move| {
                    !(m.kind == "admitted" && m.args.get("listed").and_then(Value::as_bool) == Some(false))
                        && !(m.kind == "removed" && m.args.get("quiet").and_then(Value::as_bool) == Some(true))
                        && m.kind != "keys"
                };
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
                        let last = self.moves.iter().rev().find(|x| x.actor() == m.key).map(|x| x.at);
                        let mut devices: Vec<&Key> = self.devices.iter().filter(|(_, id)| **id == m.key).map(|(k, _)| k).collect();
                        devices.sort();
                        json!({ "name": m.name, "key": m.key, "is_agent": m.is_agent, "agent_of": m.agent_of_name, "agent_of_key": m.agent_of, "joined": m.joined, "last_acted": last, "devices": devices })
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
                    "room_key": self.args.room_key, "at": self.args.at, "rules": self.args.rules, "host_id": self.args.host_id(),
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
        let args = params.arguments.clone().unwrap_or_default();
        let who = self.gate(&name, &seal, &args)?;
        let line = self.apply(&name, &args, &seal, &who, at, host, None)?;
        self.settle(&who);
        self.counters.insert(seal.key.clone(), seal.counter);
        self.notify(FEED);
        Ok(CallToolResult::success(vec![ContentBlock::text(line)]))
    }

    /// Whether this sealed key may use this verb now: the rules every call
    /// meets, live or replayed, by the room's rules number.
    fn gate(&self, name: &str, seal: &Seal, args: &JsonObject) -> Result<Member, ErrorData> {
        let host_verb = self.has_verb(name).ok_or_else(|| bad(format!("this room has no verb called {name}")))?;
        if seal.counter <= self.counters.get(&seal.key).copied().unwrap_or(0) {
            return Err(refused("this call was already made"));
        }
        let who = match self.rules {
            Rules::One => match self.members.get(&seal.key) {
                Some(m) if m.removed => return Err(refused(format!("{} was removed from this room", m.name))),
                Some(m) => m.clone(),
                // Someone let in unlisted, acting for the first time: their key matches the mark.
                None => match self.unlisted.get(&key_mark(&self.args.id, &seal.key)) {
                    Some((name, joined)) => Member { key: seal.key.clone(), name: name.clone(), is_agent: false, agent_of: None, agent_of_name: None, listed: false, joined: *joined, removed: false, account: None },
                    None => return Err(refused(NOT_A_MEMBER)),
                },
            },
            Rules::Two => self.gate_two(name, seal, args)?,
        };
        if host_verb && who.key != self.args.host_id() {
            return Err(refused("only the host may do that"));
        }
        Ok(who)
    }

    /// Rules 2: a person acts from any key on their account's current list,
    /// an agent from its own key. A key the room doesn't know may bring a
    /// device list that names it (`keys`): a member's new device, or someone
    /// let in unlisted, acting for the first time.
    fn gate_two(&self, name: &str, seal: &Seal, args: &JsonObject) -> Result<Member, ErrorData> {
        if let Some(m) = self.members.get(&self.member_id(&seal.key)) {
            if m.removed {
                return Err(refused(format!("{} was removed from this room", m.name)));
            }
            return Ok(m.clone());
        }
        if name == "keys" {
            let proof = proof_arg(args)?;
            let account = proof.check().map_err(refused)?;
            if !account.devices.contains(&seal.key) {
                return Err(refused("that device list doesn't name the key that brought it"));
            }
            match self.members.get(&account.id) {
                Some(m) if m.removed => return Err(refused(format!("{} was removed from this room", m.name))),
                Some(m) => return Ok(m.clone()),
                None => {
                    if let Some((name, joined)) = self.unlisted.get(&key_mark(&self.args.id, &account.id)) {
                        return Ok(Member { key: account.id, name: name.clone(), is_agent: false, agent_of: None, agent_of_name: None, listed: false, joined: *joined, removed: false, account: Some(proof) });
                    }
                }
            }
        }
        if let Some(m) = self.retired.get(&seal.key).and_then(|id| self.members.get(id)) {
            return Err(refused(format!("that key no longer acts for {}: a newer device list left it off", m.name)));
        }
        Err(refused(NOT_A_MEMBER))
    }

    /// One move from a record, replayed: its seal re-checked, the call gated
    /// as it would be live, and what it makes compared with what it says.
    fn replay(&mut self, m: &Move) -> Result<(), String> {
        if m.seal.key != m.by || !recheck(&self.args.id, &m.verb, &m.args, &m.seal) {
            return Err(format!("{}'s seal does not hold", m.id));
        }
        let who = self.gate(&m.verb, &m.seal, &m.args).map_err(|e| format!("{} would have been refused: {}", m.id, e.message))?;
        self.apply(&m.verb, &m.args, &m.seal, &who, m.at, &NoHost, Some(m)).map_err(|e| format!("{} does not replay: {}", m.id, e.message))?;
        self.settle(&who);
        self.counters.insert(m.seal.key.clone(), m.seal.counter);
        if self.moves.last() != Some(m) {
            return Err(format!("{} was changed after it was made", m.id));
        }
        Ok(())
    }

    /// After someone's first move lands: if they were a mark, they're a member now.
    fn settle(&mut self, who: &Member) {
        if !self.members.contains_key(&who.key) && self.unlisted.remove(&key_mark(&self.args.id, &who.key)).is_some() {
            if let Some(account) = who.account.as_ref().and_then(|p| p.check().ok()) {
                self.set_devices(&who.key, &account.devices);
            }
            self.members.insert(who.key.clone(), who.clone());
        }
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
            by: seal.key.clone(),
            member: (who.key != seal.key).then(|| who.key.clone()),
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
        s.kind == "receipt" && s.holds() && self.host_holds(&s.key) && s.field("room") == Some(self.args.id.as_str()) && s.field("task") == Some(task) && s.field("to") == Some(to)
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
            "take_offer" => {
                let offer = need(args, "offer_id")?.to_owned();
                let o = self.find(&offer, "offer")?;
                let ask = o.parent.clone().ok_or_else(|| bad("that offer isn't on an ask"))?;
                let a = self.find(&ask, "ask")?;
                if a.actor() != who.key {
                    return Err(refused("only who asked may take an offer on it"));
                }
                let state = self.state_of(&ask).unwrap_or_default();
                if state != "open" {
                    return Err(refused(format!("{ask} is {state}")));
                }
                fields.insert("offer".into(), json!(offer));
                fields.insert("offered_by".into(), json!(o.author));
                if self.args.kind == Kind::Board {
                    // The offer becomes a task its offerer already holds: the spec is what they offered.
                    let task = push(self, "task", &a.title, &o.body, Some(&ask), fields)?;
                    self.derive(&task, Some("claimed"), &[("claimed_by", json!(o.author)), ("claimed_by_key", json!(o.by))]);
                } else {
                    push(self, "taken", &a.title, &o.body, Some(&ask), fields)?;
                }
                self.derive(&ask, Some("taken"), &[("taken_offer", json!(offer))]);
                self.derive(&offer, Some("taken"), &[]);
                format!("Took {}'s offer: {}", o.author, a.title)
            }
            "close_ask" => {
                let ask = need(args, "ask_id")?.to_owned();
                let a = self.find(&ask, "ask")?;
                if a.actor() != who.key {
                    return Err(refused("only who asked may close it"));
                }
                let state = self.state_of(&ask).unwrap_or_default();
                if state != "open" {
                    return Err(refused(format!("{ask} is {state}")));
                }
                push(self, "closed", &a.title, arg(args, "note").unwrap_or_default(), Some(&ask), fields)?;
                self.derive(&ask, Some("closed"), &[]);
                format!("Closed: {}", a.title)
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
                if t.actor() != who.key {
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
                    None => {
                        let mut body = json!({ "room": self.args.id, "room_title": self.args.title, "host": self.args.host_name, "task": task, "title": t.title, "to": to_key, "to_name": to_name, "to_person": to_person, "issued": at });
                        // Under rules 2 the room's id names the host's account: the receipt carries it, so it proves its room anywhere.
                        if let Some(account) = self.members.get(&self.args.host_id()).and_then(|h| h.account.clone()) {
                            body["host_account"] = serde_json::to_value(account).unwrap_or_default();
                        }
                        host.seal("receipt", body).map_err(refused)?
                    }
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
                let side = if t.actor() == who.key {
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
                if self.member_id(statement.field("to_person").unwrap_or_default()) != self.args.host_id() {
                    return Err(refused("that receipt was issued to someone else"));
                }
                // A receipt proves its room only if it was sealed by the host that room's id names, and that isn't you.
                match receipt_issuer(&statement) {
                    Some(issuer) if issuer != self.args.host_id() && !self.host_holds(&statement.key) => {}
                    _ => return Err(refused("that receipt wasn't sealed by the host of the room it names")),
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
                if who.key == self.args.host_id() {
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
                let member_name = need(args, "name")?.to_owned();
                let is_agent = args.get("is_agent").and_then(Value::as_bool).unwrap_or(false);
                let listed = args.get("listed").and_then(Value::as_bool).unwrap_or(true);
                // Unlisted: let in by a mark of their key (under rules 2, their account), which the record keeps instead.
                if !listed {
                    if is_agent || args.contains_key("key") || args.contains_key("account") {
                        return Err(bad("someone unlisted is let in by their key's mark, never their key"));
                    }
                    let mark = need(args, "key_mark")?.to_owned();
                    if self.unlisted.contains_key(&mark) || self.members.keys().any(|k| key_mark(&self.args.id, k) == mark && !self.members[k].removed) {
                        return Err(refused(format!("{member_name} is already let in")));
                    }
                    fields.insert("key_mark".into(), json!(mark));
                    push(self, "admitted", &member_name, "", None, fields)?;
                    self.unlisted.insert(mark, (member_name.clone(), at));
                    return Ok(format!("Let in, unlisted: {member_name}"));
                }
                // Under rules 2 a person comes in as their account, an agent as its own key.
                let (key, account) = if self.rules == Rules::Two && !is_agent {
                    if args.contains_key("key") {
                        return Err(bad("under these rules a person is let in with their account, not a key"));
                    }
                    let proof = proof_arg(args)?;
                    let account = proof.check().map_err(refused)?;
                    // Someone let back in brings a list no older than the one the room last held for them.
                    if let Some(held) = self.members.get(&account.id).and_then(|m| m.account.as_ref()).and_then(|p| p.check().ok()) {
                        if account.sequence < held.sequence || (account.sequence == held.sequence && account.devices != held.devices) {
                            return Err(refused(format!("this device list is number {}, and number {} is already held", account.sequence, held.sequence)));
                        }
                    }
                    if self.taken(&account.id, &account.devices) {
                        return Err(refused("a key on that device list already acts for someone else here"));
                    }
                    (account.id.clone(), Some((proof, account.devices)))
                } else {
                    (need(args, "key")?.to_owned(), None)
                };
                let (agent_of, agent_of_name) = if is_agent {
                    // Its person, by key or (under rules 2) by account or any key on its list.
                    let person = self.member_id(need(args, "agent_of")?);
                    let tether: Statement = serde_json::from_value(args.get("tether").cloned().unwrap_or_default()).map_err(|_| bad("an agent needs its person's tether"))?;
                    if tether.kind != "tether" || !tether.holds() || self.member_id(&tether.key) != person || tether.field("agent") != Some(key.as_str()) {
                        return Err(refused("that agent's tether does not hold"));
                    }
                    if self.devices.contains_key(&key) {
                        return Err(refused("that key already acts for someone else here"));
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
                let proof = account.map(|(proof, devices)| {
                    self.set_devices(&key, &devices);
                    proof
                });
                self.members.insert(key.clone(), Member { key, name: member_name.clone(), is_agent, agent_of, agent_of_name, listed, joined: at, removed: false, account: proof });
                self.notify(MEMBERS);
                format!("Admitted: {member_name}")
            }
            "remove" => {
                // A person by key, or under rules 2 by account or any key on its list.
                let key = self.member_id(need(args, "key")?);
                if key == self.args.host_id() {
                    return Err(refused("the host can't remove themselves; end the room instead"));
                }
                // Someone let in unlisted who never acted is removed by their mark, which never names them.
                if let Some((gone, _)) = self.unlisted.get(&key).cloned() {
                    pick(&mut fields, &["reason"]);
                    fields.insert("key_mark".into(), json!(key));
                    push(self, "removed", &gone, arg(args, "reason").unwrap_or_default(), None, fields)?;
                    self.unlisted.remove(&key);
                    self.gone.insert(key);
                    self.notify(MEMBERS);
                    return Ok(format!("Removed: {gone}"));
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
            "keys" => {
                // Who brings it was settled at the gate: a key on their current list, or on this one.
                let proof = proof_arg(args)?;
                let account = proof.check().map_err(refused)?;
                if account.id != who.key {
                    return Err(refused("that device list is for another account"));
                }
                if let Some(held) = self.members.get(&who.key).and_then(|m| m.account.as_ref()).and_then(|p| p.check().ok()) {
                    if account.sequence <= held.sequence {
                        return Err(refused(format!("this device list is number {}, and number {} is already held", account.sequence, held.sequence)));
                    }
                }
                if !account.devices.contains(&seal.key) && self.member_id(&seal.key) != who.key {
                    return Err(refused("that device list doesn't name the key that brought it"));
                }
                if self.taken(&who.key, &account.devices) {
                    return Err(refused("a key on that device list already acts for someone else here"));
                }
                fields.insert("sequence".into(), json!(account.sequence));
                push(self, "keys", &who.name, "", None, fields)?;
                self.set_devices(&who.key, &account.devices);
                if let Some(m) = self.members.get_mut(&who.key) {
                    m.account = Some(proof);
                }
                self.notify(MEMBERS);
                format!("Device list number {} for {}", account.sequence, who.name)
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
