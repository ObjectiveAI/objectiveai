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
use diverge_desktop_room::{Args, Host, Key, Keypair, Kind, Room, Statement, seal_call, tether};
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
    provider: Identity,
    online: bool,
    secret: String,
    mine: bool,
    /// Whether you're in it (a room you host, you always are).
    joined: bool,
    /// For a room someone invented hosts: who.
    stand_in_host: Option<String>,
}

struct Inner {
    rooms: IndexMap<String, Hosted>,
    pending: Vec<Knock>,
    next_knock: u64,
    next_room: u64,
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

pub const BOARD: &str = "board-saturday";

pub fn obj(v: Value) -> rmcp::model::JsonObject {
    v.as_object().cloned().unwrap_or_default()
}

fn text_of(result: &CallToolResult) -> String {
    result.content.first().and_then(|c| c.as_text()).map(|t| t.text.clone()).unwrap_or_default()
}

impl StubSpaces {
    /// The records you'd hold copies of from before the app ever ran: the
    /// stand-in's past. The real app keeps its own copies as it goes.
    pub fn records_you_hold(&self) -> Vec<(String, Value)> {
        let inner = self.lock();
        inner
            .rooms
            .iter()
            .filter(|(_, h)| h.joined && !h.online)
            .map(|(id, h)| (id.clone(), json!({ "args": h.room.args, "history": [], "moves": h.room.moves() })))
            .collect()
    }

    /// Must be called inside a tokio runtime; that runtime is the one it keeps.
    /// `tables` is where each room's table lives on this Mac.
    pub fn new(me: Arc<Keys>, tables: PathBuf) -> Self {
        let (knocks_live, _) = broadcast::channel(64);
        let (calls_live, _) = broadcast::channel(64);
        let mut people = HashMap::new();
        for name in ["ada", "ada/scout", "ren"] {
            people.insert(name.to_owned(), StandIn { keypair: Keypair::from_seed(name), counter: 0 });
        }
        let s = StubSpaces {
            inner: Arc::new(Mutex::new(Inner { rooms: IndexMap::new(), pending: Vec::new(), next_knock: 1, next_room: 1, people })),
            me,
            rt: tokio::runtime::Handle::current(),
            knocks_live,
            calls_live,
            tables,
        };
        s.seed();
        s
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn stand_in_key(&self, name: &str) -> Key {
        Keypair::from_seed(name).key()
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
        let mut inner = self.lock();
        self.act(&mut inner, who, room, verb, args, Utc::now())
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

    #[allow(clippy::too_many_arguments)]
    fn open(&self, inner: &mut Inner, id: &str, title: &str, kind: Kind, host: (&str, &str), provider: Identity, charter: &str, secret: &str, open_door: bool, stand_in_host: Option<&str>) {
        let args = Args { id: id.into(), title: title.into(), kind, host_key: host.0.into(), host_name: host.1.into(), charter: charter.into(), open_door, continues: None };
        let mine = stand_in_host.is_none();
        if mine {
            self.me.set_room(id, "usual");
        }
        inner.rooms.insert(id.into(), Hosted { room: Room::new(args), provider, online: true, secret: secret.into(), mine, joined: mine, stand_in_host: stand_in_host.map(str::to_owned) });
        let _ = std::fs::create_dir_all(self.table_dir(id));
    }

    /// Admit one of your agents, tethered to you.
    fn admit_agent(&self, inner: &mut Inner, room: &str, agent: &str, at: DateTime<Utc>) {
        let (key, t) = self.me.agent_in(agent, Some(room));
        let usual = self.me.usual();
        let _ = self.me_act(inner, Actor::Persona("usual".into()), room, "admit", json!({ "key": key, "name": agent, "is_agent": true, "agent_of": usual.key, "tether": t }), at);
    }

    fn last_id(inner: &Inner, room: &str) -> String {
        inner.rooms[room].room.moves().last().map(|m| m.id.clone()).unwrap_or_default()
    }

    fn seed(&self) {
        let now = Utc::now();
        let ago = |h: i64, m: i64| now - TimeDelta::hours(h) - TimeDelta::minutes(m);
        let usual = self.me.usual();
        let me = Actor::Persona("usual".into());
        let (ada, scout, ren) = (self.stand_in_key("ada"), self.stand_in_key("ada/scout"), self.stand_in_key("ren"));
        let mut guard = self.lock();
        let inner = &mut *guard;

        // Your home.
        self.open(inner, "home-me", "Your home", Kind::Home, (&usual.key, &usual.name), mine(), HOME_CHARTER, "home-friends", false, None);
        let _ = self.me_act(inner, me.clone(), "home-me", "admit", json!({ "key": ada, "name": "ada" }), ago(200, 0));
        self.admit_agent(inner, "home-me", "research-notes", ago(48, 0));
        self.admit_agent(inner, "home-me", "site-fixes", ago(40, 0));
        let _ = self.me_act(inner, me.clone(), "home-me", "show", json!({ "title": "Draft one of the desktop app runs", "body": "Agents, storage, machines and saved Views, on a stand-in for the daemon. Screenshots tomorrow." }), ago(26, 0));
        let _ = self.me_act(inner, me.clone(), "home-me", "ask", json!({ "what": "Who has a GPU free on weekends?", "needs": "a card with 24 GB, reachable from my daemon", "ceiling": "one weekend", "who_may_serve": "anyone" }), ago(3, 10));
        let ask = Self::last_id(inner, "home-me");
        let _ = self.act(inner, "ada", "home-me", "reply", json!({ "move_id": ask, "body": "Mine's free Saturday, the studio PC. Say when and I'll issue you a key." }), ago(2, 20));
        let _ = self.me_act(inner, Actor::Agent("site-fixes".into()), "home-me", "report", json!({ "title": "Went through the site", "body": "12 passes. One broken link: “Archive” points at /old-page, which doesn't exist. Nothing changed.", "measured": "1,556 tokens" }), ago(0, 20));

        // A work board you host.
        self.open(inner, BOARD, "Saturday Workshop", Kind::Board, (&usual.key, &usual.name), mine(), BOARD_CHARTER, "saturday-2026", false, None);
        let _ = self.me_act(inner, me.clone(), BOARD, "admit", json!({ "key": ada, "name": "ada" }), ago(70, 0));
        self.admit_agent(inner, BOARD, "research-notes", ago(48, 0));
        self.admit_agent(inner, BOARD, "site-fixes", ago(40, 0));
        let _ = self.act(inner, "ada", BOARD, "post_task", json!({ "title": "Package the photo resizer as a tool", "spec": "A container that runs the resizer script as a tool with one `resize` verb. Done = an agent can call it on a folder and get the smaller copies back.", "pledge": "a loaf of the good bread" }), ago(25, 0));
        let _ = OPEN_TASK.set(Self::last_id(inner, BOARD));
        let _ = self.me_act(inner, me.clone(), BOARD, "post_task", json!({ "title": "Write a README for the label printer script", "spec": "What it does, how to run it, one example. Done = someone new can print a label from it." }), ago(24, 30));
        let readme = Self::last_id(inner, BOARD);
        let _ = self.act(inner, "ada", BOARD, "claim", json!({ "task_id": readme }), ago(24, 0));
        let _ = self.act(inner, "ada", BOARD, "post_offering", json!({ "title": "Weekend GPU time on the studio PC", "what": "One card, 24 GB, Saturday and Sunday. Measured in seconds; friends free.", "pricing": "metered", "terms": "measured, not priced" }), ago(23, 0));
        let _ = self.me_act(inner, me.clone(), BOARD, "show", json!({ "title": "The desktop app, draft one", "body": "It runs: agents, storage, machines, saved views." }), ago(22, 0));
        self.put(BOARD, "photo-resizer/README.md", "# photo resizer\n\n`resize.sh <folder>` writes a smaller copy of every photo into `<folder>/small/`.\n");
        self.put(BOARD, "photo-resizer/resize.sh", "#!/bin/sh\nmkdir -p \"$1/small\"\nfor f in \"$1\"/*.jpg; do sips -Z 1600 \"$f\" --out \"$1/small/\"; done\n");
        self.put(BOARD, "label-printer/print-label.py", "import sys\nprint(f\"printing: {sys.argv[1]}\")\n");

        // An idea room ada hosts; you're in it as yourself.
        self.open(inner, "idea-ada", "A zine for the neighbourhood", Kind::Idea, (&ada, "ada"), ada_machine(), IDEA_CHARTER, "ideas-open", false, Some("ada"));
        let scout_tether = tether(&inner.people["ada"].keypair, &scout, "scout");
        let _ = self.act(inner, "ada", "idea-ada", "admit", json!({ "key": scout, "name": "scout", "is_agent": true, "agent_of": ada, "tether": scout_tether }), ago(85, 0));
        let _ = self.act(inner, "ada", "idea-ada", "admit", json!({ "key": usual.key, "name": usual.name }), ago(80, 0));
        self.me.set_room("idea-ada", "usual");
        if let Some(h) = inner.rooms.get_mut("idea-ada") {
            h.joined = true;
        }
        let _ = self.act(inner, "ada", "idea-ada", "propose", json!({ "direction": "Recipes from every street", "body": "One page per street, cooked by whoever lives there." }), ago(70, 0));
        let d1 = Self::last_id(inner, "idea-ada");
        let _ = self.me_act(inner, me.clone(), "idea-ada", "propose", json!({ "direction": "A map drawn by kids", "body": "Hand-drawn, scanned, the centre spread." }), ago(60, 0));
        let d2 = Self::last_id(inner, "idea-ada");
        let _ = self.act(inner, "ada", "idea-ada", "steer", json!({ "direction_id": d2, "move": "prefer", "note": "This is the one. Recipes can be issue two." }), ago(55, 0));
        let _ = self.act(inner, "ada/scout", "idea-ada", "steer", json!({ "direction_id": d1, "move": "note", "note": "Recipe zines are common; a map drawn by kids is rarer." }), ago(54, 0));
        let _ = self.act(inner, "ada", "idea-ada", "synthesize", json!({ "body": "Issue one is the kids' map; recipes wait for issue two." }), ago(40, 0));
        let workshop = self.invite_for(inner, BOARD).map(|i| i.to_text()).unwrap_or_default();
        let _ = self.act(inner, "ada", "idea-ada", "vouch_room", json!({ "title": "Saturday Workshop", "invite": workshop }), ago(39, 0));
        self.put("idea-ada", "map-sketch.txt", "north: the bakery, the bridge, the school\nsouth: the park, the pond, the bus stop\n");

        // A room ada hosted on her old laptop, gone quiet. You hold a copy of its record.
        self.open(inner, CAFE, "Tuesday repair café", Kind::Board, (&ada, "ada"), Identity::Outgoing { address: "ada-old-laptop.local:4640".into() }, CAFE_CHARTER, "cafe", false, Some("ada"));
        let _ = self.act(inner, "ada", CAFE, "admit", json!({ "key": usual.key, "name": usual.name }), ago(400, 0));
        self.me.set_room(CAFE, "usual");
        let _ = self.act(inner, "ada", CAFE, "post_task", json!({ "title": "Fix the café's toaster", "spec": "Both slots heat. Done = two slices, evenly brown.", "pledge": "free coffee for a month" }), ago(390, 0));
        let toaster = Self::last_id(inner, CAFE);
        let _ = self.me_act(inner, me.clone(), CAFE, "claim", json!({ "task_id": toaster }), ago(380, 0));
        let _ = self.me_act(inner, me.clone(), CAFE, "deliver", json!({ "task_id": toaster, "summary": "New element in the left slot. Both slots heat now." }), ago(370, 0));
        let _ = self.act(inner, "ada", CAFE, "accept", json!({ "task_id": toaster }), ago(369, 0));
        let _ = self.act(inner, "ada", CAFE, "show", json!({ "title": "Last Tuesday of the season", "body": "Thanks all. I'm moving the café to my new machine soon." }), ago(340, 0));
        if let Some(h) = inner.rooms.get_mut(CAFE) {
            h.joined = true;
            h.online = false;
        }

        // ren's music room: joinable with an invite ada passes on.
        self.open(inner, "music-ren", "Ren's music room", Kind::Home, (&ren, "ren"), ren_machine(), "Tracks I make. No AI. Say what you hear.", "ren-open", false, Some("ren"));
        let _ = self.act(inner, "ren", "music-ren", "show", json!({ "title": "New background track for a vid", "body": "Mixed with the lows under 120 Hz kept down so a voice sits over it." }), ago(9, 0));
        let ren_invite = self.invite_for(inner, "music-ren").map(|i| i.to_text()).unwrap_or_default();

        // A direct room with ada.
        self.open(inner, "dm-ada", "ada", Kind::Dm, (&usual.key, &usual.name), mine(), "Two people. Nothing leaves this room.", "dm", false, None);
        let _ = self.me_act(inner, me.clone(), "dm-ada", "admit", json!({ "key": ada, "name": "ada" }), ago(200, 0));
        let _ = self.act(inner, "ada", "dm-ada", "say", json!({ "body": "did the draft run?" }), ago(5, 0));
        let _ = self.me_act(inner, me.clone(), "dm-ada", "say", json!({ "body": "yes, screenshots tomorrow" }), ago(4, 50));
        let _ = self.act(inner, "ada", "dm-ada", "say", json!({ "body": format!("ren makes music, you'd like the room. here's the way in:\n\n{ren_invite}") }), ago(4, 40));

        // Your profile: a room anyone with the link can knock on.
        self.open(inner, "profile-me", &usual.name, Kind::Profile, (&usual.key, &usual.name), mine(), PROFILE_CHARTER, "profile-open", true, None);
        let _ = self.me_act(inner, me.clone(), "profile-me", "show", json!({ "title": "The desktop app, draft one", "body": "Agents, storage, machines and saved Views, on a stand-in for the daemon." }), ago(26, 0));
        let _ = self.me_act(inner, me.clone(), "profile-me", "post_offering", json!({ "title": "A site check-up", "what": "site-fixes goes through a small site and lists what's broken. Nothing is changed without asking.", "pricing": "fixed", "terms": "you get a list, in a day" }), ago(20, 0));
        let _ = self.me_act(inner, me.clone(), "profile-me", "admit", json!({ "key": ren, "name": "ren" }), ago(8, 0));
        let _ = self.act(inner, "ren", "profile-me", "leave_note", json!({ "body": "Loved the kids' map idea in ada's zine room." }), ago(7, 30));

        // ren at the workshop's door, with an invite and a note.
        let vouch = Statement::make(&inner.people["ada"].keypair, "vouch", json!({ "for": ren, "for_name": "ren", "by_name": "ada" }));
        let knocking = Knocking { secret: Some("saturday-2026".into()), key: ren.clone(), name: "ren".into(), note: "ada said to come by. I fix lamps and radios.".into(), listed: true, vouch: Some(vouch) };
        inner.pending.push(Knock {
            knock_id: 1,
            space: Id { id: BOARD.into() },
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
            let _ = this.act_now("ren", "profile-me", "hire", json!({ "agent": "site-fixes", "what": "Check the links on my music page", "pledge": "a copy of the next track" }));
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
        let args = json!({ "key": knocking.key, "name": knocking.name, "listed": knocking.listed });
        match self.act(&mut inner, &host, id, "admit", args, Utc::now()) {
            Ok(_) => {
                if let Some(h) = inner.rooms.get_mut(id) {
                    h.joined = true;
                }
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
        // A successor carries the record of the room it continues.
        let history: Vec<diverge_desktop_room::Move> = container.arguments.get("history").cloned().and_then(|h| serde_json::from_value(h).ok()).unwrap_or_default();
        let mut args: Args = serde_json::from_value(container.arguments).map_err(|e| wire_error(format!("these are not a room's settings: {e}")))?;
        if args.title.trim().is_empty() {
            return Err(wire_error("a Space needs a title"));
        }
        let mut inner = self.lock();
        let id = format!("space-{}", inner.next_room);
        inner.next_room += 1;
        args.id = id.clone();
        let host = RoomHost { stand_in: None, me: &self.me, host_key: args.host_key.clone(), calls: self.calls_live.clone() };
        let room = if args.continues.is_some() { Room::from_record(args, history, &[], &host).map_err(wire_error)? } else { Room::new(args) };
        let secret = format!("{id}-{}", &diverge_desktop_room::seal::digest(format!("{id}{}", Utc::now()).as_bytes())[..8]);
        inner.rooms.insert(id.clone(), Hosted { room, provider: mine(), online: true, secret, mine: true, joined: true, stand_in_host: None });
        let _ = std::fs::create_dir_all(self.table_dir(&id));
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

    async fn answer(&self, knock_id: u64, _answer: Answer) -> Result<Knock, String> {
        let mut inner = self.lock();
        let at = inner.pending.iter().position(|k| k.knock_id == knock_id).ok_or("nobody is at that door any more")?;
        Ok(inner.pending.remove(at))
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
            if knocking.secret.as_deref() != Some(h.secret.as_str()) && !h.room.args.open_door {
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
        let (args, moves) = (h.room.args.clone(), h.room.moves().to_vec());
        let host = RoomHost { stand_in: None, me: &self.me, host_key: args.host_key.clone(), calls: self.calls_live.clone() };
        let room = Room::from_record(args, Vec::new(), &moves, &host).map_err(wire_error)?;
        if let Some(h) = inner.rooms.get_mut(&id.id) {
            h.room = room;
        }
        Ok(())
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
pub const CAFE: &str = "board-cafe";
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
            diverge_desktop_room::check_record(id, h.room.moves()).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert!(!h.room.moves().is_empty(), "{id} has a history");
        }
        let task = inner.rooms[BOARD].room.moves().iter().find(|m| m.id == open_task()).unwrap().clone();
        assert_eq!((task.kind.as_str(), task.author.as_str()), ("task", "ada"));
        let report = inner.rooms["home-me"].room.moves().iter().find(|m| m.kind == "run").unwrap().clone();
        assert_eq!((report.author.as_str(), report.agent_of.as_deref()), ("site-fixes", Some("maya")), "an agent's move names its person");
    }

    #[tokio::test]
    async fn a_knock_let_in_by_the_host_then_the_newcomer_speaks() {
        let (spaces, me) = stub();
        let mut knocks = spaces.knocks(CancellationToken::new());
        let knock = knocks.next().await.unwrap();
        let knocking = Knocking::from_authorization(&knock.authorize.authorization).unwrap();
        assert_eq!(knocking.name, "ren");
        assert!(spaces.act_now("ren", BOARD, "show", json!({ "title": "early" })).is_err(), "not in yet");
        spaces.answer(knock.knock_id, Answer::Authorized).await.unwrap();
        let admit = sealed(&me, Actor::Persona("usual".into()), BOARD, "admit", json!({ "key": knocking.key, "name": knocking.name }));
        spaces.call(&knock.space, admit).await.unwrap();
        spaces.act_now("ren", BOARD, "show", json!({ "title": "a radio I fixed" })).unwrap();
        assert!(feed(&spaces, BOARD).iter().any(|m| m["author"] == "ren" && m["kind"] == "show"));
    }

    #[tokio::test(start_paused = true)]
    async fn an_invite_passed_on_in_a_dm_lets_you_into_rens_room() {
        let (spaces, me) = stub();
        let said = feed(&spaces, "dm-ada").into_iter().filter_map(|m| m["body"].as_str().map(str::to_owned)).find(|b| b.contains("diverge-invite:")).unwrap();
        let invite = Invite::from_text(said.split_whitespace().find(|w| w.starts_with("diverge-invite:")).unwrap()).unwrap();
        assert_eq!(invite.title, "Ren's music room");
        assert!(invite.verbs.iter().any(|v| v.name == "show"));
        let usual = me.usual();
        let wrong = Knocking { secret: Some("guess".into()), key: usual.key.clone(), name: usual.name.clone(), note: String::new(), listed: true, vouch: None };
        assert!(matches!(spaces.join(&invite, &wrong).await, Joined::Denied));
        let knocking = Knocking { secret: invite.secret.clone(), ..wrong };
        let Joined::Joined(id) = spaces.join(&invite, &knocking).await else { panic!("let in") };
        me.set_room(&id.id, "usual");
        spaces.call(&id, sealed(&me, Actor::Persona("usual".into()), &id.id, "show", json!({ "title": "hello" }))).await.unwrap();
        assert!(feed(&spaces, "music-ren").iter().any(|m| m["author"] == "maya" && m["title"] == "hello"));
        let about = spaces.read(&id, program::ABOUT).await.unwrap();
        let ResourceContents::TextResourceContents { text, .. } = &about.contents[0] else { panic!() };
        let about: Value = serde_json::from_str(text).unwrap();
        assert_eq!(about["charter"], diverge_desktop_room::seal::fingerprint(&invite.charter), "the room's rules are the ones the invite showed");
    }

    #[tokio::test]
    async fn the_table_is_shared_and_files_move_only_within_one_machine() {
        let (spaces, _) = stub();
        let board = Id { id: BOARD.into() };
        let names: Vec<String> = spaces.table_tree(&board).await.unwrap().iter().map(|n| crate::daemon::stub::store::name_of(n).to_owned()).collect();
        assert!(names.contains(&"photo-resizer".to_owned()));
        spaces.table_write(&board, &["notes".into(), "saturday.md".into()], b"bring the soldering iron".to_vec()).await.unwrap();
        assert_eq!(spaces.table_read(&board, &["notes".into(), "saturday.md".into()]).await.unwrap(), b"bring the soldering iron");
        assert!(spaces.table_write(&board, &["..".into(), "escape".into()], b"x".to_vec()).await.is_err());
        let home = Id { id: "home-me".into() };
        spaces.transfer(&board, &["notes".into(), "saturday.md".into()], &home).await.unwrap();
        assert!(spaces.table_read(&home, &["notes".into(), "saturday.md".into()]).await.is_ok());
        let zine = Id { id: "idea-ada".into() };
        assert!(spaces.transfer(&board, &["notes".into(), "saturday.md".into()], &zine).await.is_err(), "ada's room runs on her machine");
        assert!(spaces.table_tree(&Id { id: "music-ren".into() }).await.is_err(), "not in ren's room");
    }

    #[tokio::test]
    async fn a_hire_through_your_profile_reaches_you() {
        let (spaces, _) = stub();
        let mut calls = spaces.host_calls(CancellationToken::new());
        spaces.act_now("ren", "profile-me", "hire", json!({ "agent": "site-fixes", "what": "check my links", "pledge": "a coffee" })).unwrap();
        let Some(HostCall::Hire { from, agent, .. }) = calls.next().await else { panic!("heard") };
        assert_eq!((from.as_str(), agent.as_str()), ("ren", "site-fixes"));
    }
}
