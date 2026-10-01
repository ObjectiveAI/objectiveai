//! The stand-in for Spaces: the real room program (`diverge-desktop-room`),
//! run in process, with two invented people: ada, whose machine is
//! `studio-pc`, and ren, who dials in. Everyone seals their own calls: you
//! and your agents through the app's keys, ada and ren through stand-in
//! keys of their own. Each room has a table: a real folder, as a room
//! container's files would be. Same answers the wire would give.

use std::collections::HashMap;
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use futures::stream;
use indexmap::IndexMap;
use rmcp::model::{CallToolRequestParams, CallToolResult, ErrorData, ListToolsResult, ReadResourceResult, ServerNotification};
use serde_json::{Value, json};
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

use diverge_desktop_room::room::META_HOST_ONLY;
use diverge_desktop_room::{Args, Host, Key, Keypair, Kind, Move, Record, Room, Statement, room_id, seal_call, tether};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::shared::error::Error as WireError;
use diverge_sdk::shared::filetree::response::Node;

use super::{Answer, Authorize, Container, HostCall, Id, Invite, InviteVerb, Joined, Knock, Knocking, SpaceEntry, Spaces};
use crate::daemon::Frames;
use crate::identity::{Actor, Identity as Keys};

/// Your own provider, as the stand-in daemon names it.
pub fn mine() -> Identity {
    Identity::Outgoing { address: "127.0.0.1:4640".into() }
}

fn ada_machine() -> Identity {
    Identity::IncomingUnbrokered { identity: "studio-pc".into() }
}

fn ren_machine() -> Identity {
    Identity::Outgoing { address: "ren-laptop.local:4640".into() }
}

fn wire_error(message: impl Into<String>) -> WireError {
    WireError(json!({ "message": message.into() }))
}

/// Someone invented, with a key of their own.
struct StandIn {
    keypair: Keypair,
    counter: u64,
}

struct Hosted {
    room: Room,
    /// The key the room countersigns with. The program holds it; the stand-in is the program.
    room_key: Keypair,
    provider: Identity,
    online: bool,
    secret: String,
    mine: bool,
    /// Whether you're in it (a room you host, you always are).
    joined: bool,
    /// For a room someone invented hosts: who.
    stand_in_host: Option<String>,
}

/// What the stand-in keeps between launches: each room's whole record and
/// its key, and who is at the door. At launch the rooms are rebuilt by
/// replaying their records, as a real host restarts a room.
#[derive(serde::Serialize, serde::Deserialize)]
struct Saved {
    rooms: Vec<SavedRoom>,
    labels: HashMap<String, String>,
    pending: Vec<Knock>,
    next_knock: u64,
    open_task: Option<String>,
    board: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedRoom {
    record: Record,
    room_secret: String,
    provider: Identity,
    online: bool,
    secret: String,
    mine: bool,
    joined: bool,
    stand_in_host: Option<String>,
}

/// The stand-in's rooms file: every room it runs, kept between launches.
pub const ROOMS: crate::store::Format = crate::store::Format { name: "stand-in rooms", version: 1, keep_previous: true };

/// Present in any build the stand-in is part of: the check that it's absent looks for these words.
pub const BUILT_IN: &str = "diverge-desktop stand-in rooms are built in";

struct Inner {
    rooms: IndexMap<String, Hosted>,
    pending: Vec<Knock>,
    next_knock: u64,
    /// The stand-in's rooms by the label they were seeded with: tests and scenes name them so.
    labels: HashMap<String, String>,
    people: HashMap<String, StandIn>,
}

#[derive(Clone)]
pub struct StubSpaces {
    inner: Arc<Mutex<Inner>>,
    me: Arc<Keys>,
    rt: tokio::runtime::Handle,
    knocks_live: broadcast::Sender<Knock>,
    calls_live: broadcast::Sender<HostCall>,
    tables: PathBuf,
}

/// The host's side of a room, for the room program: seal receipts, hear hires.
struct RoomHost<'a> {
    stand_in: Option<Keypair>,
    me: &'a Keys,
    host_key: Key,
    calls: broadcast::Sender<HostCall>,
}

impl Host for RoomHost<'_> {
    fn seal(&self, kind: &str, body: Value) -> Result<Statement, String> {
        match &self.stand_in {
            Some(k) => Ok(Statement::make(k, kind, body)),
            None => self.me.state(&self.host_key, kind, body),
        }
    }

    fn hire(&self, room: &str, hire_id: &str, from: &str, agent: &str, what: &str, pledge: Option<&str>) {
        let _ = self.calls.send(HostCall::Hire {
            room: Id { id: room.into() },
            hire_id: hire_id.into(),
            from: from.into(),
            agent: agent.into(),
            what: what.into(),
            pledge: pledge.map(str::to_owned),
        });
    }
}

static OPEN_TASK: OnceLock<String> = OnceLock::new();

/// The seeded open task on the Saturday Workshop board: the scripted
/// "claim a task" run names it.
pub fn open_task() -> String {
    OPEN_TASK.get().cloned().unwrap_or_else(|| "task-1".into())
}

static BOARD_ID: OnceLock<String> = OnceLock::new();

/// The Saturday Workshop board you host in the stand-in: the scripted
/// "claim a task" run names it.
pub fn board() -> String {
    BOARD_ID.get().cloned().unwrap_or_default()
}

pub fn obj(v: Value) -> rmcp::model::JsonObject {
    v.as_object().cloned().unwrap_or_default()
}

fn text_of(result: &CallToolResult) -> String {
    result.content.first().and_then(|c| c.as_text()).map(|t| t.text.clone()).unwrap_or_default()
}

impl StubSpaces {
    /// The records you'd hold copies of from before the app ever ran: the
    /// stand-in's past. The real app keeps its own copies as it goes.
    pub fn records_you_hold(&self) -> Vec<(String, Record)> {
        let inner = self.lock();
        inner.rooms.iter().filter(|(_, h)| h.joined && !h.online).map(|(id, h)| (id.clone(), h.room.record())).collect()
    }

    /// Must be called inside a tokio runtime; that runtime is the one it keeps.
    /// `tables` is where each room's table lives on this Mac.
    pub fn new(me: Arc<Keys>, tables: PathBuf) -> Self {
        std::hint::black_box(BUILT_IN);
        let (knocks_live, _) = broadcast::channel(64);
        let (calls_live, _) = broadcast::channel(64);
        let mut people = HashMap::new();
        for name in ["ada", "ada/scout", "ren"] {
            people.insert(name.to_owned(), StandIn { keypair: Keypair::from_seed(name), counter: 0 });
        }
        let s = StubSpaces {
            inner: Arc::new(Mutex::new(Inner { rooms: IndexMap::new(), pending: Vec::new(), next_knock: 1, labels: HashMap::new(), people })),
            me,
            rt: tokio::runtime::Handle::current(),
            knocks_live,
            calls_live,
            tables,
        };
        // What happened in the rooms last time, if anything did; the stand-in's own past otherwise,
        // once there's a you: until the first-run page is finished, nothing is seeded.
        if !s.restore() {
            s.begin();
        }
        s
    }

    /// Seed the stand-in's own past, if there are no rooms yet and the
    /// first-run page is finished: the seeded rooms are yours too.
    pub fn begin(&self) {
        if self.me.ready().is_err() || !self.lock().rooms.is_empty() {
            return;
        }
        self.seed();
        self.persist();
    }

    fn saved_file(&self) -> PathBuf {
        self.tables.join(".stand-in-rooms.json")
    }

    /// Keep every room as it stands now: after anything changes.
    fn persist(&self) {
        let inner = self.lock();
        let saved = Saved {
            rooms: inner
                .rooms
                .values()
                .map(|h| SavedRoom {
                    record: h.room.record(),
                    room_secret: h.room_key.secret_hex(),
                    provider: h.provider.clone(),
                    online: h.online,
                    secret: h.secret.clone(),
                    mine: h.mine,
                    joined: h.joined,
                    stand_in_host: h.stand_in_host.clone(),
                })
                .collect(),
            labels: inner.labels.clone(),
            pending: inner.pending.clone(),
            next_knock: inner.next_knock,
            open_task: OPEN_TASK.get().cloned(),
            board: BOARD_ID.get().cloned(),
        };
        drop(inner);
        let _ = crate::store::save(&self.saved_file(), ROOMS, &saved);
    }

    /// Every kept room, rebuilt by replaying its record.
    fn replay(saved: Saved) -> Result<(IndexMap<String, Hosted>, Saved), String> {
        let mut rooms = IndexMap::new();
        for r in &saved.rooms {
            let key = Keypair::from_secret_hex(&r.room_secret)?;
            let room = Room::from_record(r.record.clone(), Some(key.clone()))?;
            let id = room.id().to_owned();
            rooms.insert(id, Hosted { room, room_key: key, provider: r.provider.clone(), online: r.online, secret: r.secret.clone(), mine: r.mine, joined: r.joined, stand_in_host: r.stand_in_host.clone() });
        }
        Ok((rooms, saved))
    }

    /// Rebuild the rooms from what was kept, every record replayed. A file
    /// that won't parse or replay is set aside, and the stand-in starts
    /// again from its own past.
    fn restore(&self) -> bool {
        let check = |saved: &Saved| saved.rooms.iter().try_for_each(|r| Keypair::from_secret_hex(&r.room_secret).and_then(|k| Room::from_record(r.record.clone(), Some(k)).map(|_| ())));
        let Some(saved) = crate::store::load_with::<Saved>(&self.saved_file(), ROOMS, check) else { return false };
        let Ok((rooms, saved)) = Self::replay(saved) else { return false };
        let mut inner = self.lock();
        // The invented people carry on counting from where their seals got to.
        for person in inner.people.values_mut() {
            let key = person.keypair.key();
            person.counter = rooms.values().flat_map(|h| h.room.moves().iter()).filter(|m| m.by == key).map(|m| m.seal.counter).max().unwrap_or(0);
        }
        inner.rooms = rooms;
        inner.labels = saved.labels;
        inner.pending = saved.pending;
        inner.next_knock = saved.next_knock;
        drop(inner);
        if let Some(t) = saved.open_task {
            let _ = OPEN_TASK.set(t);
        }
        if let Some(b) = saved.board {
            let _ = BOARD_ID.set(b);
        }
        true
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn stand_in_key(&self, name: &str) -> Key {
        Keypair::from_seed(name).key()
    }

    /// A seeded room's id, by the label it was seeded with.
    pub fn id_of(&self, label: &str) -> String {
        self.lock().labels.get(label).cloned().unwrap_or_else(|| label.to_owned())
    }

    /// One sealed call into a room, with its host's side answering.
    fn call_in(&self, inner: &mut Inner, room: &str, params: CallToolRequestParams, at: DateTime<Utc>) -> Result<CallToolResult, ErrorData> {
        let stand_in = inner.rooms.get(room).and_then(|h| h.stand_in_host.clone()).and_then(|n| inner.people.get(&n)).map(|p| p.keypair.clone());
        let hosted = inner.rooms.get_mut(room).ok_or_else(|| ErrorData::invalid_request("no such room", None))?;
        let host = RoomHost { stand_in, me: &self.me, host_key: hosted.room.args.host_key.clone(), calls: self.calls_live.clone() };
        hosted.room.call_at(params, at, &host)
    }

    /// Someone invented, acting in a room.
    fn act(&self, inner: &mut Inner, who: &str, room: &str, verb: &str, args: Value, at: DateTime<Utc>) -> Result<String, String> {
        let person = inner.people.get_mut(who).ok_or("nobody by that name")?;
        person.counter += 1;
        let mut params = CallToolRequestParams::new(verb.to_owned()).with_arguments(obj(args));
        seal_call(&person.keypair, room, &mut params, person.counter);
        self.call_in(inner, room, params, at).map(|r| text_of(&r)).map_err(|e| e.message.to_string())
    }

    /// Someone invented, acting now: for the stand-in's own scenes and tests.
    pub fn act_now(&self, who: &str, room: &str, verb: &str, args: Value) -> Result<String, String> {
        let out = {
            let mut inner = self.lock();
            self.act(&mut inner, who, room, verb, args, Utc::now())
        };
        self.persist();
        out
    }

    /// You (or one of your agents) acting in a room, in the stand-in's history.
    fn me_act(&self, inner: &mut Inner, actor: Actor, room: &str, verb: &str, args: Value, at: DateTime<Utc>) -> Result<String, String> {
        let mut params = CallToolRequestParams::new(verb.to_owned()).with_arguments(obj(args));
        self.me.seal(&actor, room, &mut params)?;
        self.call_in(inner, room, params, at).map(|r| text_of(&r)).map_err(|e| e.message.to_string())
    }

    fn table_dir(&self, room: &str) -> PathBuf {
        self.tables.join(room)
    }

    fn table_path(&self, room: &str, path: &[String]) -> Result<PathBuf, WireError> {
        let mut out = self.table_dir(room);
        for part in path {
            match Path::new(part).components().next() {
                Some(Component::Normal(_)) if !part.contains('/') => out.push(part),
                _ => return Err(wire_error(format!("\"{part}\" is not a name on a table"))),
            }
        }
        Ok(out)
    }

    fn put(&self, room: &str, path: &str, body: &str) {
        let p = self.table_dir(room).join(path);
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if !p.exists() {
            let _ = std::fs::write(p, body);
        }
    }

    /// Open a seeded room: its id is its label and its host's mark, its
    /// settings signed by its host, its key the stand-in's to hold.
    #[allow(clippy::too_many_arguments)]
    fn open(&self, inner: &mut Inner, label: &str, title: &str, kind: Kind, host: (&str, &str), provider: Identity, charter: &str, secret: &str, open_door: bool, stand_in_host: Option<&str>, since: DateTime<Utc>) -> String {
        let id = room_id(label, host.0);
        let room_key = Keypair::from_seed(&format!("stand-in room {id}"));
        let mut args = Args {
            id: id.clone(),
            title: title.into(),
            kind,
            host_key: host.0.into(),
            host_name: host.1.into(),
            charter: charter.into(),
            open_door,
            continues: None,
            room_key: room_key.key(),
            at: since,
            rules: 1,
            host_account: None,
            sig: String::new(),
        };
        args = match stand_in_host.and_then(|n| inner.people.get(n)) {
            Some(p) => args.signed(&p.keypair),
            None => {
                let sig = self.me.state(host.0, "room", args.body()).map(|s| s.sig).unwrap_or_default();
                Args { sig, ..args }
            }
        };
        let mine = stand_in_host.is_none();
        if mine {
            self.me.set_room(&id, "usual");
        }
        let room = Room::new(args, room_key.clone()).expect("a seeded room's settings hold");
        inner.rooms.insert(id.clone(), Hosted { room, room_key, provider, online: true, secret: secret.into(), mine, joined: mine, stand_in_host: stand_in_host.map(str::to_owned) });
        inner.labels.insert(label.into(), id.clone());
        let _ = std::fs::create_dir_all(self.table_dir(&id));
        id
    }

    /// Admit one of your agents, tethered to you.
    fn admit_agent(&self, inner: &mut Inner, room: &str, agent: &str, at: DateTime<Utc>) {
        let (Ok(a), Ok(usual)) = (self.me.agent_in(agent, Some(room)), self.me.usual()) else { return };
        let (key, t, agent) = (a.key, a.tether, a.name.as_str());
        let _ = self.me_act(inner, Actor::Persona("usual".into()), room, "admit", json!({ "key": key, "name": agent, "is_agent": true, "agent_of": usual.key, "tether": t }), at);
    }

    fn last_id(inner: &Inner, room: &str) -> String {
        inner.rooms[room].room.moves().last().map(|m| m.id.clone()).unwrap_or_default()
    }

    fn seed(&self) {
        let now = Utc::now();
        let ago = |h: i64, m: i64| now - TimeDelta::hours(h) - TimeDelta::minutes(m);
        // Every seeded room was made before anything happened in it.
        let since = ago(500, 0);
        let Ok(usual) = self.me.usual() else { return };
        let me = Actor::Persona("usual".into());
        let (ada, scout, ren) = (self.stand_in_key("ada"), self.stand_in_key("ada/scout"), self.stand_in_key("ren"));
        let mut guard = self.lock();
        let inner = &mut *guard;

        // Your home.
        let home = self.open(inner, "home-me", "Your home", Kind::Home, (&usual.key, &usual.name), mine(), HOME_CHARTER, "home-friends", false, None, since);
        let _ = self.me_act(inner, me.clone(), &home, "admit", json!({ "key": ada, "name": "ada" }), ago(200, 0));
        self.admit_agent(inner, &home, "research-notes", ago(48, 0));
        self.admit_agent(inner, &home, "site-fixes", ago(40, 0));
        let _ = self.me_act(inner, me.clone(), &home, "show", json!({ "title": "Draft one of the desktop app runs", "body": "Agents, storage, machines and saved Views, on a stand-in for the daemon. Screenshots tomorrow." }), ago(26, 0));
        let _ = self.me_act(inner, me.clone(), &home, "ask", json!({ "what": "Who has a GPU free on weekends?", "needs": "a card with 24 GB, reachable from my daemon", "ceiling": "one weekend", "who_may_serve": "anyone" }), ago(3, 10));
        let ask = Self::last_id(inner, &home);
        let _ = self.act(inner, "ada", &home, "offer", json!({ "ask_id": ask, "body": "Mine's free Saturday, the studio PC. Say when and I'll issue you a key." }), ago(2, 20));
        let _ = self.me_act(inner, Actor::Agent("site-fixes".into()), &home, "report", json!({ "title": "Went through the site", "body": "12 passes. One broken link: “Archive” points at /old-page, which doesn't exist. Nothing changed.", "measured": "1,556 tokens" }), ago(0, 20));

        // A work board you host.
        let board = self.open(inner, "board-saturday", "Saturday Workshop", Kind::Board, (&usual.key, &usual.name), mine(), BOARD_CHARTER, "saturday-2026", false, None, since);
        let _ = self.me_act(inner, me.clone(), &board, "admit", json!({ "key": ada, "name": "ada" }), ago(70, 0));
        self.admit_agent(inner, &board, "research-notes", ago(48, 0));
        self.admit_agent(inner, &board, "site-fixes", ago(40, 0));
        let _ = self.act(inner, "ada", &board, "post_task", json!({ "title": "Package the photo resizer as a tool", "spec": "A container that runs the resizer script as a tool with one `resize` verb. Done = an agent can call it on a folder and get the smaller copies back.", "pledge": "a loaf of the good bread" }), ago(25, 0));
        let _ = OPEN_TASK.set(Self::last_id(inner, &board));
        let _ = BOARD_ID.set(board.clone());
        let _ = self.me_act(inner, me.clone(), &board, "post_task", json!({ "title": "Write a README for the label printer script", "spec": "What it does, how to run it, one example. Done = someone new can print a label from it." }), ago(24, 30));
        let readme = Self::last_id(inner, &board);
        let _ = self.act(inner, "ada", &board, "claim", json!({ "task_id": readme }), ago(24, 0));
        let _ = self.act(inner, "ada", &board, "post_offering", json!({ "title": "Weekend GPU time on the studio PC", "what": "One card, 24 GB, Saturday and Sunday. Measured in seconds; friends free.", "pricing": "metered", "terms": "measured, not priced" }), ago(23, 0));
        let _ = self.me_act(inner, me.clone(), &board, "show", json!({ "title": "The desktop app, draft one", "body": "It runs: agents, storage, machines, saved views." }), ago(22, 0));
        self.put(&board, "photo-resizer/README.md", "# photo resizer\n\n`resize.sh <folder>` writes a smaller copy of every photo into `<folder>/small/`.\n");
        self.put(&board, "photo-resizer/resize.sh", "#!/bin/sh\nmkdir -p \"$1/small\"\nfor f in \"$1\"/*.jpg; do sips -Z 1600 \"$f\" --out \"$1/small/\"; done\n");
        self.put(&board, "label-printer/print-label.py", "import sys\nprint(f\"printing: {sys.argv[1]}\")\n");

        // An idea room ada hosts; you're in it as yourself.
        let idea = self.open(inner, "idea-ada", "A zine for the neighbourhood", Kind::Idea, (&ada, "ada"), ada_machine(), IDEA_CHARTER, "ideas-open", false, Some("ada"), since);
        let scout_tether = tether(&inner.people["ada"].keypair, &scout, "scout");
        let _ = self.act(inner, "ada", &idea, "admit", json!({ "key": scout, "name": "scout", "is_agent": true, "agent_of": ada, "tether": scout_tether }), ago(85, 0));
        let _ = self.act(inner, "ada", &idea, "admit", json!({ "key": usual.key, "name": usual.name }), ago(80, 0));
        self.me.set_room(&idea, "usual");
        if let Some(h) = inner.rooms.get_mut(&idea) {
            h.joined = true;
        }
        let _ = self.act(inner, "ada", &idea, "propose", json!({ "direction": "Recipes from every street", "body": "One page per street, cooked by whoever lives there." }), ago(70, 0));
        let d1 = Self::last_id(inner, &idea);
        let _ = self.me_act(inner, me.clone(), &idea, "propose", json!({ "direction": "A map drawn by kids", "body": "Hand-drawn, scanned, the centre spread." }), ago(60, 0));
        let d2 = Self::last_id(inner, &idea);
        let _ = self.act(inner, "ada", &idea, "steer", json!({ "direction_id": d2, "move": "prefer", "note": "This is the one. Recipes can be issue two." }), ago(55, 0));
        let _ = self.act(inner, "ada/scout", &idea, "steer", json!({ "direction_id": d1, "move": "note", "note": "Recipe zines are common; a map drawn by kids is rarer." }), ago(54, 0));
        let _ = self.act(inner, "ada", &idea, "synthesize", json!({ "body": "Issue one is the kids' map; recipes wait for issue two." }), ago(40, 0));
        let workshop = self.invite_for(inner, &board).map(|i| i.to_text()).unwrap_or_default();
        let _ = self.act(inner, "ada", &idea, "vouch_room", json!({ "title": "Saturday Workshop", "invite": workshop }), ago(39, 0));
        self.put(&idea, "map-sketch.txt", "north: the bakery, the bridge, the school\nsouth: the park, the pond, the bus stop\n");

        // A room ada hosted on her old laptop, gone quiet. You hold a copy of its record.
        let cafe = self.open(inner, "board-cafe", "Tuesday repair café", Kind::Board, (&ada, "ada"), Identity::Outgoing { address: "ada-old-laptop.local:4640".into() }, CAFE_CHARTER, "cafe", false, Some("ada"), since);
        let _ = self.act(inner, "ada", &cafe, "admit", json!({ "key": usual.key, "name": usual.name }), ago(400, 0));
        self.me.set_room(&cafe, "usual");
        let _ = self.act(inner, "ada", &cafe, "post_task", json!({ "title": "Fix the café's toaster", "spec": "Both slots heat. Done = two slices, evenly brown.", "pledge": "free coffee for a month" }), ago(390, 0));
        let toaster = Self::last_id(inner, &cafe);
        let _ = self.me_act(inner, me.clone(), &cafe, "claim", json!({ "task_id": toaster }), ago(380, 0));
        let _ = self.me_act(inner, me.clone(), &cafe, "deliver", json!({ "task_id": toaster, "summary": "New element in the left slot. Both slots heat now." }), ago(370, 0));
        let _ = self.act(inner, "ada", &cafe, "accept", json!({ "task_id": toaster }), ago(369, 0));
        let _ = self.act(inner, "ada", &cafe, "show", json!({ "title": "Last Tuesday of the season", "body": "Thanks all. I'm moving the café to my new machine soon." }), ago(340, 0));
        if let Some(h) = inner.rooms.get_mut(&cafe) {
            h.joined = true;
            h.online = false;
        }

        // ren's music room: joinable with an invite ada passes on.
        let music = self.open(inner, "music-ren", "Ren's music room", Kind::Home, (&ren, "ren"), ren_machine(), "Tracks I make. No AI. Say what you hear.", "ren-open", false, Some("ren"), since);
        let _ = self.act(inner, "ren", &music, "show", json!({ "title": "New background track for a vid", "body": "Mixed with the lows under 120 Hz kept down so a voice sits over it." }), ago(9, 0));
        let ren_invite = self.invite_for(inner, &music).map(|i| i.to_text()).unwrap_or_default();

        // A direct room with ada.
        let dm = self.open(inner, "dm-ada", "ada", Kind::Dm, (&usual.key, &usual.name), mine(), "Two people. Nothing leaves this room.", "dm", false, None, since);
        let _ = self.me_act(inner, me.clone(), &dm, "admit", json!({ "key": ada, "name": "ada" }), ago(200, 0));
        let _ = self.act(inner, "ada", &dm, "say", json!({ "body": "did the draft run?" }), ago(5, 0));
        let _ = self.me_act(inner, me.clone(), &dm, "say", json!({ "body": "yes, screenshots tomorrow" }), ago(4, 50));
        let _ = self.act(inner, "ada", &dm, "say", json!({ "body": format!("ren makes music, you'd like the room. here's the way in:\n\n{ren_invite}") }), ago(4, 40));

        // Your profile: a room anyone with the link can knock on.
        let profile = self.open(inner, "profile-me", &usual.name, Kind::Profile, (&usual.key, &usual.name), mine(), PROFILE_CHARTER, "profile-open", true, None, since);
        let _ = self.me_act(inner, me.clone(), &profile, "show", json!({ "title": "The desktop app, draft one", "body": "Agents, storage, machines and saved Views, on a stand-in for the daemon." }), ago(26, 0));
        let _ = self.me_act(inner, me.clone(), &profile, "post_offering", json!({ "title": "A site check-up", "what": "site-fixes goes through a small site and lists what's broken. Nothing is changed without asking.", "pricing": "fixed", "terms": "you get a list, in a day" }), ago(20, 0));
        let _ = self.me_act(inner, me.clone(), &profile, "admit", json!({ "key": ren, "name": "ren" }), ago(8, 0));
        let _ = self.act(inner, "ren", &profile, "leave_note", json!({ "body": "Loved the kids' map idea in ada's zine room." }), ago(7, 30));

        // ren at the workshop's door, with an invite and a note.
        let until = (now + super::VOUCH_GOOD_FOR).to_rfc3339();
        let vouch = Statement::make(&inner.people["ada"].keypair, "vouch", json!({ "for": ren, "for_name": "ren", "by_name": "ada", "room": board, "until": until }));
        let knocking = Knocking {
            room: board.clone(),
            invite: Some(Knocking::invite_mark(&board, "saturday-2026")),
            key: ren.clone(),
            name: "ren".into(),
            note: "ada said to come by. I fix lamps and radios.".into(),
            listed: true,
            vouch: Some(vouch),
            account: None,
            at: ago(0, 2),
            sig: String::new(),
        }
        .signed(&inner.people["ren"].keypair);
        inner.pending.push(Knock {
            knock_id: 1,
            space: Id { id: board.clone() },
            authorize: Authorize { address: "10.0.0.42".parse::<IpAddr>().unwrap(), authorization: knocking.to_authorization() },
            at: ago(0, 2),
        });
        inner.next_knock = 2;
    }

    /// The stand-in's scenes, once the app is listening: ren hires one of
    /// your agents through your profile a few seconds in.
    pub fn stage(&self) {
        let this = self.clone();
        self.rt.spawn(async move {
            tokio::time::sleep(Duration::from_secs(8)).await;
            let profile = this.id_of("profile-me");
            let _ = this.act_now("ren", &profile, "hire", json!({ "agent": "site-fixes", "what": "Check the links on my music page", "pledge": "a copy of the next track" }));
        });
    }

    /// ada answers an ask a few seconds later: a second person in the room.
    fn ada_replies(&self, id: String, ask: String) {
        let this = self.clone();
        self.rt.spawn(async move {
            tokio::time::sleep(Duration::from_secs(4)).await;
            let mut inner = this.lock();
            let ada = this.stand_in_key("ada");
            if inner.rooms.get(&id).is_some_and(|h| h.room.member(&ada).is_some_and(|m| !m.removed)) {
                let _ = this.act(&mut inner, "ada", &id, "offer", json!({ "ask_id": ask, "body": "I can look at that tomorrow. Say a bit more about what done looks like?" }), Utc::now());
            }
            drop(inner);
            this.persist();
        });
    }

    fn invite_for(&self, inner: &Inner, id: &str) -> Option<Invite> {
        let h = inner.rooms.get(id)?;
        let verbs = h
            .room
            .tools()
            .tools
            .iter()
            .filter(|t| !t.meta.as_ref().and_then(|m| m.0.get(META_HOST_ONLY)).and_then(Value::as_bool).unwrap_or(false))
            .map(|t| InviteVerb { name: t.name.to_string(), does: t.description.as_deref().unwrap_or_default().to_owned() })
            .collect();
        Some(Invite {
            host: h.provider.clone(),
            id: id.into(),
            secret: Some(h.secret.clone()),
            title: h.room.args.title.clone(),
            kind: h.room.args.kind.key().into(),
            host_name: h.room.args.host_name.clone(),
            charter: h.room.charter().into(),
            verbs,
        })
    }

    /// The stand-in's other side of a knock: someone invented decides, then
    /// admits the joiner under the name they gave, sealed with their own key.
    async fn decide(&self, id: &str, knocking: &Knocking) -> Joined {
        tokio::time::sleep(Duration::from_millis(1500)).await;
        let mut inner = self.lock();
        let Some(host) = inner.rooms.get(id).and_then(|h| h.stand_in_host.clone()) else { return Joined::Missing };
        let rules = inner.rooms.get(id).map(|h| h.room.rules()).unwrap_or(diverge_desktop_room::Rules::One);
        let Some(args) = crate::actions::admit_args(rules, id, knocking) else { return Joined::Denied };
        match self.act(&mut inner, &host, id, "admit", args, Utc::now()) {
            Ok(_) => {
                if let Some(h) = inner.rooms.get_mut(id) {
                    h.joined = true;
                }
                drop(inner);
                self.persist();
                Joined::Joined(Id { id: id.into() })
            }
            Err(e) => Joined::Error(wire_error(e)),
        }
    }

    /// A table is reachable while you're in its room and the room is up.
    fn reachable(&self, id: &Id) -> Result<(), WireError> {
        let inner = self.lock();
        let h = inner.rooms.get(&id.id).ok_or_else(|| wire_error("no such room"))?;
        if !(h.mine || h.joined) {
            return Err(wire_error("you're not in that room"));
        }
        if !h.online {
            return Err(wire_error("the room's host is offline"));
        }
        Ok(())
    }
}

#[async_trait]
impl Spaces for StubSpaces {
    async fn list(&self) -> Vec<SpaceEntry> {
        let inner = self.lock();
        inner
            .rooms
            .iter()
            .filter(|(_, h)| h.mine || h.joined)
            .map(|(id, h)| SpaceEntry {
                id: Id { id: id.clone() },
                title: h.room.args.title.clone(),
                kind: h.room.args.kind.key().into(),
                host: h.provider.clone(),
                host_name: h.room.args.host_name.clone(),
                host_key: h.room.args.host_key.clone(),
                mine: h.mine,
                online: h.online,
            })
            .collect()
    }

    async fn host(&self, container: Container) -> Result<Id, WireError> {
        // As the room program's /register takes them: signed settings, the
        // room's key, and any record (a successor's `before`, a restart's moves).
        let mut arguments = container.arguments;
        let obj = arguments.as_object_mut().ok_or_else(|| wire_error("a room's settings are an object"))?;
        let secret = obj.remove("room_secret").and_then(|v| v.as_str().map(str::to_owned)).ok_or_else(|| wire_error("a room needs its key"))?;
        let room_key = Keypair::from_secret_hex(&secret).map_err(wire_error)?;
        let before: Option<Record> = match obj.remove("before") {
            None | Some(Value::Null) => None,
            Some(v) => Some(serde_json::from_value(v).map_err(|e| wire_error(format!("the record it continues is damaged: {e}")))?),
        };
        let moves: Vec<Move> = match obj.remove("moves") {
            None | Some(Value::Null) => Vec::new(),
            Some(v) => serde_json::from_value(v).map_err(|e| wire_error(format!("its record is damaged: {e}")))?,
        };
        let args: Args = serde_json::from_value(arguments).map_err(|e| wire_error(format!("these are not a room's settings: {e}")))?;
        if args.title.trim().is_empty() {
            return Err(wire_error("a Space needs a title"));
        }
        let id = args.id.clone();
        let room = Room::from_record(Record { args, before: before.map(Box::new), moves }, Some(room_key.clone())).map_err(wire_error)?;
        let mut inner = self.lock();
        if inner.rooms.contains_key(&id) {
            return Err(wire_error("a room with that id already runs here"));
        }
        let secret = diverge_desktop_room::fresh_label();
        inner.rooms.insert(id.clone(), Hosted { room, room_key, provider: mine(), online: true, secret, mine: true, joined: true, stand_in_host: None });
        let _ = std::fs::create_dir_all(self.table_dir(&id));
        drop(inner);
        self.persist();
        Ok(Id { id })
    }

    fn knocks(&self, cancel: CancellationToken) -> Frames<Knock> {
        let (tx, rx) = mpsc::channel::<Knock>(64);
        let pending = self.lock().pending.clone();
        let mut live = self.knocks_live.subscribe();
        self.rt.spawn(async move {
            for k in pending {
                if tx.send(k).await.is_err() {
                    return;
                }
            }
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    next = live.recv() => match next {
                        Ok(k) => { if tx.send(k).await.is_err() { return; } }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => return,
                    },
                }
            }
        });
        Box::pin(stream::unfold(rx, |mut rx| async move { rx.recv().await.map(|k| (k, rx)) }))
    }

    async fn pending(&self, knock_id: u64) -> Option<Knock> {
        self.lock().pending.iter().find(|k| k.knock_id == knock_id).cloned()
    }

    async fn answer(&self, knock_id: u64, _answer: Answer) -> Result<Knock, String> {
        let knock = {
            let mut inner = self.lock();
            let at = inner.pending.iter().position(|k| k.knock_id == knock_id).ok_or("nobody is at that door any more")?;
            inner.pending.remove(at)
        };
        self.persist();
        Ok(knock)
    }

    async fn join(&self, invite: &Invite, knocking: &Knocking) -> Joined {
        {
            let inner = self.lock();
            let Some(h) = inner.rooms.get(&invite.id) else { return Joined::Missing };
            if h.mine {
                return Joined::Error(wire_error("you host that room"));
            }
            if h.provider != invite.host {
                return Joined::Missing;
            }
            // The stand-in's hosts check a knock as the app does: signed by its key, for this room, and invited or at an open door.
            let invited = knocking.invite.as_deref() == Some(Knocking::invite_mark(&invite.id, &h.secret).as_str());
            if knocking.check(&invite.id, Utc::now()).is_err() || !(invited || h.room.args.open_door) {
                return Joined::Denied;
            }
        }
        self.decide(&invite.id, knocking).await
    }

    async fn leave(&self, id: &Id) -> Result<(), String> {
        let mut inner = self.lock();
        let h = inner.rooms.get_mut(&id.id).ok_or("no such room")?;
        if h.mine {
            inner.rooms.shift_remove(&id.id);
        } else {
            h.joined = false;
        }
        drop(inner);
        self.persist();
        Ok(())
    }

    async fn tools(&self, id: &Id) -> Result<ListToolsResult, ErrorData> {
        let inner = self.lock();
        inner.rooms.get(&id.id).map(|h| h.room.tools()).ok_or_else(|| ErrorData::invalid_request("no such room", None))
    }

    async fn read(&self, id: &Id, uri: &str) -> Result<ReadResourceResult, ErrorData> {
        let inner = self.lock();
        let h = inner.rooms.get(&id.id).ok_or_else(|| ErrorData::invalid_request("no such room", None))?;
        if !h.online {
            return Err(ErrorData::internal_error("the room's host is offline", None));
        }
        // Only someone let in and not removed reads a room: here, you, as whoever you are there.
        let you = self.me.who_in(&id.id).ok();
        let may = |who: Option<&str>| who.is_some_and(|w| h.room.may_read(w));
        if !h.mine && !may(you.as_ref().map(|p| p.key.as_str())) && !may(you.as_ref().and_then(|p| p.account.as_deref())) {
            return Err(ErrorData::invalid_request("you're not in that room, or you were removed", None));
        }
        h.room.read(uri)
    }

    async fn call(&self, id: &Id, params: CallToolRequestParams) -> Result<CallToolResult, ErrorData> {
        let verb = params.name.to_string();
        let caller = diverge_desktop_room::seal::seal_of(&params).map(|s| s.key);
        let (result, last) = {
            let mut inner = self.lock();
            let h = inner.rooms.get(&id.id).ok_or_else(|| ErrorData::invalid_request("no such room", None))?;
            if !h.online {
                return Err(ErrorData::internal_error("the room's host is offline", None));
            }
            let result = self.call_in(&mut inner, &id.id, params, Utc::now())?;
            let last = inner.rooms.get(&id.id).and_then(|h| h.room.moves().last().map(|m| m.id.clone()));
            (result, last)
        };
        self.persist();
        if verb == "ask" && caller.as_deref() != Some(self.stand_in_key("ada").as_str()) {
            if let Some(ask) = last {
                self.ada_replies(id.id.clone(), ask);
            }
        }
        Ok(result)
    }

    fn notifications(&self, id: &Id, cancel: CancellationToken) -> Frames<ServerNotification> {
        let (tx, rx) = mpsc::channel::<ServerNotification>(64);
        let Some(mut live) = self.lock().rooms.get(&id.id).map(|h| h.room.subscribe()) else {
            return Box::pin(stream::empty());
        };
        self.rt.spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    next = live.recv() => match next {
                        Ok(n) => { if tx.send(n).await.is_err() { return; } }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => return,
                    },
                }
            }
        });
        Box::pin(stream::unfold(rx, |mut rx| async move { rx.recv().await.map(|n| (n, rx)) }))
    }

    async fn invite(&self, id: &Id) -> Option<Invite> {
        let inner = self.lock();
        if !inner.rooms.get(&id.id)?.mine {
            return None;
        }
        self.invite_for(&inner, &id.id)
    }

    async fn home(&self) -> Option<Id> {
        let inner = self.lock();
        inner.rooms.iter().find(|(_, h)| h.mine && h.room.args.kind == Kind::Home).map(|(id, _)| Id { id: id.clone() })
    }

    async fn profile(&self) -> Option<Id> {
        let inner = self.lock();
        inner.rooms.iter().find(|(_, h)| h.mine && h.room.args.kind == Kind::Profile).map(|(id, _)| Id { id: id.clone() })
    }

    fn host_calls(&self, cancel: CancellationToken) -> Frames<HostCall> {
        let (tx, rx) = mpsc::channel::<HostCall>(64);
        let mut live = self.calls_live.subscribe();
        self.rt.spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    next = live.recv() => match next {
                        Ok(c) => { if tx.send(c).await.is_err() { return; } }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => return,
                    },
                }
            }
        });
        Box::pin(stream::unfold(rx, |mut rx| async move { rx.recv().await.map(|c| (c, rx)) }))
    }

    async fn table_tree(&self, id: &Id) -> Result<Vec<Node>, WireError> {
        self.reachable(id)?;
        Ok(crate::daemon::stub::store::entries(&self.table_dir(&id.id)))
    }

    async fn table_read(&self, id: &Id, path: &[String]) -> Result<Vec<u8>, WireError> {
        self.reachable(id)?;
        let p = self.table_path(&id.id, path)?;
        std::fs::read(&p).map_err(|_| wire_error(format!("nothing at /{} on this table", path.join("/"))))
    }

    async fn table_write(&self, id: &Id, path: &[String], body: Vec<u8>) -> Result<(), WireError> {
        self.reachable(id)?;
        if path.is_empty() {
            return Err(wire_error("a file needs a name"));
        }
        let p = self.table_path(&id.id, path)?;
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| wire_error(e.to_string()))?;
        }
        std::fs::write(&p, body).map_err(|e| wire_error(e.to_string()))
    }

    async fn restart(&self, id: &Id) -> Result<(), WireError> {
        let mut inner = self.lock();
        let h = inner.rooms.get(&id.id).ok_or_else(|| wire_error("no such room"))?;
        if !h.mine {
            return Err(wire_error("only its host restarts a room"));
        }
        let room = Room::from_record(h.room.record(), Some(h.room_key.clone())).map_err(wire_error)?;
        if let Some(h) = inner.rooms.get_mut(&id.id) {
            h.room = room;
        }
        drop(inner);
        self.persist();
        Ok(())
    }

    async fn restart_with_new_invite(&self, id: &Id) -> Result<Invite, WireError> {
        self.restart(id).await?;
        let invite = {
            let mut inner = self.lock();
            let h = inner.rooms.get_mut(&id.id).ok_or_else(|| wire_error("no such room"))?;
            h.secret = diverge_desktop_room::fresh_label();
            self.invite_for(&inner, &id.id).ok_or_else(|| wire_error("no such room"))?
        };
        self.persist();
        Ok(invite)
    }

    async fn transfer(&self, from: &Id, path: &[String], to: &Id) -> Result<(), WireError> {
        self.reachable(from)?;
        self.reachable(to)?;
        {
            let inner = self.lock();
            let (a, b) = (inner.rooms.get(&from.id).map(|h| h.provider.clone()), inner.rooms.get(&to.id).map(|h| h.provider.clone()));
            if a != b {
                return Err(wire_error("those rooms run on different machines; a file moves between them through your Mac instead"));
            }
        }
        let src = self.table_path(&from.id, path)?;
        let dst = self.table_path(&to.id, path)?;
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).map_err(|e| wire_error(e.to_string()))?;
        }
        std::fs::copy(src, dst).map(|_| ()).map_err(|e| wire_error(e.to_string()))
    }
}

const HOME_CHARTER: &str = "# Your home\n\nWhat you're building, shown when you choose to. Friends may ask and answer. Your agents post what they finished.";
const BOARD_CHARTER: &str = "# Saturday Workshop\n\n## Rules\nShow what you're making, finished or not. Ask for help plainly. Be kind about other people's work.\n\n## Who does what\nThe **host** keeps the rules and accepts deliveries. **Members** post, claim and offer. **Guests** read.\n\n## Tasks\nA task has a title and a spec, fixed when it's posted. Claim it as posted; the spec is what gets checked. A pledge is words: both sides say when it's settled.\n\n## Receipts\nAccepting a delivery issues a receipt to whoever did the work, sealed by this room. It is a record, never a rating of a person.";
const IDEA_CHARTER: &str = "# A zine for the neighbourhood\n\nPropose directions, steer them (prefer, reject, note), and write where it stands. Each round starts from the last synthesis.";
const CAFE_CHARTER: &str = "# Tuesday repair café\n\nBring something broken. Post it as a task; whoever fixes it claims it. Pledges are words, settled between you.";
const PROFILE_CHARTER: &str = "# My profile\n\nWhat I've made and what I offer. Leave a note, or ask one of my agents for something: I decide, and it runs on my machine.";

#[cfg(test)]
mod tests {
    use super::*;
    use diverge_desktop_room::room as program;
    use futures::StreamExt;
    use rmcp::model::ResourceContents;

    pub fn stub() -> (StubSpaces, Arc<Keys>) {
        let me = Arc::new(Keys::stand_in("maya"));
        let tables = std::env::temp_dir().join(format!("diverge-desktop-tables-{}-{}", std::process::id(), Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        (StubSpaces::new(me.clone(), tables), me)
    }

    fn feed(spaces: &StubSpaces, id: &str) -> Vec<Value> {
        let inner = spaces.lock();
        let r = inner.rooms[id].room.read(program::FEED).unwrap();
        let ResourceContents::TextResourceContents { text, .. } = &r.contents[0] else { panic!() };
        serde_json::from_str(text).unwrap()
    }

    fn sealed(me: &Keys, actor: Actor, room: &str, verb: &'static str, args: Value) -> CallToolRequestParams {
        let mut params = CallToolRequestParams::new(verb).with_arguments(obj(args));
        me.seal(&actor, room, &mut params).unwrap();
        params
    }

    #[tokio::test]
    async fn every_seeded_record_checks_out() {
        let (spaces, _) = stub();
        let inner = spaces.lock();
        for (id, h) in &inner.rooms {
            Room::check(&h.room.record()).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert!(!h.room.moves().is_empty(), "{id} has a history");
        }
        let task = inner.rooms[&board()].room.moves().iter().find(|m| m.id == open_task()).unwrap().clone();
        assert_eq!((task.kind.as_str(), task.author.as_str()), ("task", "ada"));
        let report = inner.rooms[&inner.labels["home-me"]].room.moves().iter().find(|m| m.kind == "run").unwrap().clone();
        assert_eq!((report.author.as_str(), report.agent_of.as_deref()), ("site-fixes", Some("maya")), "an agent's move names its person");
    }

    #[tokio::test]
    async fn a_knock_let_in_by_the_host_then_the_newcomer_speaks() {
        let (spaces, me) = stub();
        let mut knocks = spaces.knocks(CancellationToken::new());
        let knock = knocks.next().await.unwrap();
        let knocking = Knocking::from_authorization(&knock.authorize.authorization).unwrap();
        assert_eq!(knocking.name, "ren");
        let board = board();
        assert!(spaces.act_now("ren", &board, "show", json!({ "title": "early" })).is_err(), "not in yet");
        spaces.answer(knock.knock_id, Answer::Authorized).await.unwrap();
        let admit = sealed(&me, Actor::Persona("usual".into()), &board, "admit", json!({ "key": knocking.key, "name": knocking.name }));
        spaces.call(&knock.space, admit).await.unwrap();
        spaces.act_now("ren", &board, "show", json!({ "title": "a radio I fixed" })).unwrap();
        assert!(feed(&spaces, &board).iter().any(|m| m["author"] == "ren" && m["kind"] == "show"));
    }

    #[tokio::test(start_paused = true)]
    async fn an_invite_passed_on_in_a_dm_lets_you_into_rens_room() {
        let (spaces, me) = stub();
        let said = feed(&spaces, &spaces.id_of("dm-ada")).into_iter().filter_map(|m| m["body"].as_str().map(str::to_owned)).find(|b| b.contains("diverge-invite:")).unwrap();
        let invite = Invite::from_text(said.split_whitespace().find(|w| w.starts_with("diverge-invite:")).unwrap()).unwrap();
        assert_eq!(invite.title, "Ren's music room");
        assert!(invite.verbs.iter().any(|v| v.name == "show"));
        let usual = me.usual().unwrap();
        let knock = |invite_mark: Option<String>| Knocking {
            room: invite.id.clone(),
            invite: invite_mark,
            key: usual.key.clone(),
            name: usual.name.clone(),
            note: String::new(),
            listed: true,
            vouch: None,
            account: None,
            at: Utc::now(),
            sig: String::new(),
        };
        let sign = |k: Knocking| Knocking { sig: me.state(&usual.key, "knock", k.body()).unwrap().sig, ..k };
        let right = Knocking::invite_mark(&invite.id, invite.secret.as_deref().unwrap());
        assert!(matches!(spaces.join(&invite, &sign(knock(Some("guess".into())))).await, Joined::Denied), "a wrong invite");
        assert!(matches!(spaces.join(&invite, &knock(Some(right.clone()))).await, Joined::Denied), "unsigned");
        let stolen = Knocking { key: spaces.stand_in_key("ada"), ..sign(knock(Some(right.clone()))) };
        assert!(matches!(spaces.join(&invite, &stolen).await, Joined::Denied), "signed by someone else");
        let knocking = sign(knock(Some(right)));
        let Joined::Joined(id) = spaces.join(&invite, &knocking).await else { panic!("let in") };
        me.set_room(&id.id, "usual");
        spaces.call(&id, sealed(&me, Actor::Persona("usual".into()), &id.id, "show", json!({ "title": "hello" }))).await.unwrap();
        assert!(feed(&spaces, &spaces.id_of("music-ren")).iter().any(|m| m["author"] == "maya" && m["title"] == "hello"));
        let about = spaces.read(&id, program::ABOUT).await.unwrap();
        let ResourceContents::TextResourceContents { text, .. } = &about.contents[0] else { panic!() };
        let about: Value = serde_json::from_str(text).unwrap();
        assert_eq!(about["charter"], diverge_desktop_room::seal::fingerprint(&invite.charter), "the room's rules are the ones the invite showed");
    }

    #[tokio::test]
    async fn the_table_is_shared_and_files_move_only_within_one_machine() {
        let (spaces, _) = stub();
        let board = Id { id: board() };
        let names: Vec<String> = spaces.table_tree(&board).await.unwrap().iter().map(|n| crate::daemon::stub::store::name_of(n).to_owned()).collect();
        assert!(names.contains(&"photo-resizer".to_owned()));
        spaces.table_write(&board, &["notes".into(), "saturday.md".into()], b"bring the soldering iron".to_vec()).await.unwrap();
        assert_eq!(spaces.table_read(&board, &["notes".into(), "saturday.md".into()]).await.unwrap(), b"bring the soldering iron");
        assert!(spaces.table_write(&board, &["..".into(), "escape".into()], b"x".to_vec()).await.is_err());
        let home = Id { id: spaces.id_of("home-me") };
        spaces.transfer(&board, &["notes".into(), "saturday.md".into()], &home).await.unwrap();
        assert!(spaces.table_read(&home, &["notes".into(), "saturday.md".into()]).await.is_ok());
        let zine = Id { id: spaces.id_of("idea-ada") };
        assert!(spaces.transfer(&board, &["notes".into(), "saturday.md".into()], &zine).await.is_err(), "ada's room runs on her machine");
        assert!(spaces.table_tree(&Id { id: spaces.id_of("music-ren") }).await.is_err(), "not in ren's room");
    }

    #[tokio::test]
    async fn a_knock_and_its_vouch_hold_for_one_room_for_a_while() {
        let (spaces, _) = stub();
        let knock = spaces.knocks(CancellationToken::new()).next().await.unwrap();
        let w = Knocking::from_authorization(&knock.authorize.authorization).unwrap();
        let (board, home, now) = (board(), spaces.id_of("home-me"), Utc::now());
        assert!(w.check(&board, now).is_ok());
        assert!(w.check(&home, now).is_err(), "carried to another room");
        assert!(w.check(&board, now + TimeDelta::days(2)).is_err(), "too old");
        let changed = Knocking { name: "ada".into(), ..w.clone() };
        assert!(changed.check(&board, now).is_err(), "changed after it was signed");
        let v = w.vouch.clone().unwrap();
        assert!(crate::spaces::vouch_holds(&v, &w.key, &board, now));
        assert!(!crate::spaces::vouch_holds(&v, &w.key, &home, now), "the vouch names the workshop");
        assert!(!crate::spaces::vouch_holds(&v, &spaces.stand_in_key("ada"), &board, now), "and ren");
        assert!(!crate::spaces::vouch_holds(&v, &w.key, &board, now + TimeDelta::days(8)), "and runs out");
    }

    #[tokio::test]
    async fn what_happened_in_the_rooms_is_there_next_launch() {
        let me = Arc::new(Keys::stand_in("maya"));
        let tables = std::env::temp_dir().join(format!("diverge-desktop-relaunch-{}-{}", std::process::id(), Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        let first = StubSpaces::new(me.clone(), tables.clone());
        let board = board();
        first.act_now("ada", &board, "show", json!({ "title": "a shelf I built" })).unwrap();
        let before = feed(&first, &board);
        drop(first);
        let again = StubSpaces::new(me.clone(), tables.clone());
        assert_eq!(feed(&again, &board), before, "the same room, replayed from its record");
        again.act_now("ada", &board, "show", json!({ "title": "and a stool" })).unwrap();
        assert!(feed(&again, &board).iter().any(|m| m["title"] == "and a stool"), "ada's counter carries on");
        let _ = std::fs::remove_dir_all(&tables);
    }

    #[tokio::test]
    async fn a_rooms_file_that_wont_parse_is_set_aside_not_written_over() {
        let me = Arc::new(Keys::stand_in("maya"));
        let tables = std::env::temp_dir().join(format!("diverge-desktop-rooms-damaged-{}-{}", std::process::id(), Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        std::fs::create_dir_all(&tables).unwrap();
        std::fs::write(tables.join(".stand-in-rooms.json"), "{ \"file\": \"stand-in rooms\", \"vers").unwrap();
        let spaces = StubSpaces::new(me, tables.clone());
        assert!(!spaces.list().await.is_empty(), "the stand-in starts again from its own past");
        let aside = crate::store::notices_under(&tables);
        assert_eq!((aside.len(), aside[0].kind, aside[0].carried_on), (1, "stand-in rooms", crate::store::CarriedOn::Empty));
        assert_eq!(std::fs::read_to_string(aside[0].kept_as().unwrap()).unwrap(), "{ \"file\": \"stand-in rooms\", \"vers", "kept as it was");
        let _ = std::fs::remove_dir_all(&tables);
    }

    #[tokio::test]
    async fn a_restart_keeps_the_invite_and_a_restart_to_shut_someone_out_makes_a_new_one() {
        let (spaces, _) = stub();
        let board = Id { id: board() };
        let before = spaces.invite(&board).await.unwrap();
        let feed_before = feed(&spaces, &board.id);
        // Rebuilt from its record: the same room, the same invite.
        spaces.restart(&board).await.unwrap();
        assert_eq!(spaces.invite(&board).await.unwrap(), before, "the same invite");
        assert_eq!(feed(&spaces, &board.id), feed_before, "the same room");
        // Restarted on purpose: a new secret, and the invite says so.
        let after = spaces.restart_with_new_invite(&board).await.unwrap();
        assert_ne!(after.secret, before.secret);
        assert!(after.secret.is_some());
        assert_eq!(spaces.invite(&board).await.unwrap(), after, "the room's invite is the new one");
        assert_eq!(feed(&spaces, &board.id), feed_before, "and still the same room");
        // A knock with the invite from before no longer comes with this room's invite; one with the new one does.
        let ren = Keypair::from_seed("ren");
        let knock = |secret: &str| {
            let k = Knocking { room: board.id.clone(), invite: Some(Knocking::invite_mark(&board.id, secret)), key: ren.key(), name: "ren".into(), note: String::new(), listed: true, vouch: None, account: None, at: Utc::now(), sig: String::new() }.signed(&ren);
            Knock { knock_id: 9, space: board.clone(), authorize: Authorize { address: "10.0.0.42".parse::<IpAddr>().unwrap(), authorization: k.to_authorization() }, at: Utc::now() }
        };
        let now_secret = after.secret.as_deref();
        assert!(!crate::actions::knock_view_of(&knock(before.secret.as_deref().unwrap()), "Saturday Workshop".into(), now_secret, &[], Utc::now()).invited, "the old invite");
        assert!(crate::actions::knock_view_of(&knock(after.secret.as_deref().unwrap()), "Saturday Workshop".into(), now_secret, &[], Utc::now()).invited, "the new one");
        // Kept across a launch: the new invite, not the old.
        let tables = spaces.tables.clone();
        drop(spaces);
        let again = StubSpaces::new(Arc::new(Keys::stand_in("maya")), tables);
        assert_eq!(again.invite(&board).await.unwrap().secret, after.secret);
        // Only the host restarts a room.
        assert!(again.restart_with_new_invite(&Id { id: again.id_of("idea-ada") }).await.is_err());
    }

    #[tokio::test]
    async fn a_hire_through_your_profile_reaches_you() {
        let (spaces, _) = stub();
        let mut calls = spaces.host_calls(CancellationToken::new());
        spaces.act_now("ren", &spaces.id_of("profile-me"), "hire", json!({ "agent": "site-fixes", "what": "check my links", "pledge": "a coffee" })).unwrap();
        let Some(HostCall::Hire { from, agent, .. }) = calls.next().await else { panic!("heard") };
        assert_eq!((from.as_str(), agent.as_str()), ("ren", "site-fixes"));
    }
}
