//! The stand-in for Spaces: rooms kept in process, with two invented
//! people (ada, whose machine is `studio-pc`; ren, who dials in) so every
//! flow — knock, let in, join, post, claim — can be seen before Ronald's
//! daemon relays tool rooms. Same answers the wire would give, same types.

pub mod rooms;

use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{TimeDelta, Utc};
use futures::stream;
use indexmap::IndexMap;
use rmcp::model::{CallToolRequestParams, CallToolResult, ErrorData, ListToolsResult, ReadResourceResult, ServerNotification};
use serde_json::{Map, Value, json};
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::shared::error::Error as WireError;

use super::{Answer, Authorize, Caller, Connect, Container, Id, Invite, Joined, Knock, SpaceEntry, Spaces};
use crate::daemon::Frames;
use rooms::{Kind, Room, ME};

struct Inner {
    rooms: IndexMap<String, Room>,
    joined: HashSet<String>,
    pending: Vec<Knock>,
    next_knock: u64,
    next_room: u64,
}

#[derive(Clone)]
pub struct StubSpaces {
    inner: Arc<Mutex<Inner>>,
    rt: tokio::runtime::Handle,
    knocks_live: broadcast::Sender<Knock>,
}

pub fn mine() -> Identity {
    Identity::Outgoing { address: "127.0.0.1:4640".into() }
}

fn ada() -> Identity {
    Identity::IncomingUnbrokered { identity: "studio-pc".into() }
}

fn ren() -> Identity {
    Identity::Outgoing { address: "ren-laptop.local:4640".into() }
}

fn f(pairs: &[(&str, Value)]) -> Map<String, Value> {
    pairs.iter().map(|(k, v)| ((*k).to_owned(), v.clone())).collect()
}

impl StubSpaces {
    /// Must be called inside a tokio runtime; that runtime is the one it keeps.
    pub fn new() -> Self {
        let (knocks_live, _) = broadcast::channel(64);
        let s = StubSpaces {
            inner: Arc::new(Mutex::new(Inner { rooms: IndexMap::new(), joined: HashSet::new(), pending: Vec::new(), next_knock: 1, next_room: 1 })),
            rt: tokio::runtime::Handle::current(),
            knocks_live,
        };
        s.seed();
        s
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn seed(&self) {
        let now = Utc::now();
        let ago = |h: i64, m: i64| now - TimeDelta::hours(h) - TimeDelta::minutes(m);
        let mut inner = self.lock();

        let mut home = Room::new("home-me", "Your home", Kind::Home, mine(), true, HOME_CHARTER, "home-friends", ME);
        home.member(ME, false, ago(240, 0)).member("ada", false, ago(200, 0)).member("site-fixes", true, ago(40, 0)).member("research-notes", true, ago(48, 0));
        home.push(ago(26, 0), ME, "show", "Draft one of the desktop app runs", "Agents, storage, machines and saved Views, on a stand-in for the daemon. Screenshots tomorrow.", "shown", None, Map::new());
        let ask = home.push(ago(3, 10), ME, "ask", "Who has a GPU free on weekends?", "", "open", None, f(&[("needs", json!("a card with 24 GB, reachable from my daemon")), ("ceiling", json!("one weekend")), ("who_may_serve", json!("anyone"))]));
        home.push(ago(2, 20), "ada", "reply", "", "Mine's free Saturday — the studio PC. Say when and I'll issue you a key.", "said", Some(&ask), Map::new());
        home.push(ago(0, 20), "site-fixes", "run", "Went through the site", "12 passes. One broken link: “Archive” points at /old-page, which doesn't exist. Nothing changed.", "done", None, f(&[("measured", json!("1,556 tokens"))]));
        inner.rooms.insert(home.id.clone(), home);

        let mut board = Room::new("board-saturday", "Saturday Workshop", Kind::Board, mine(), true, BOARD_CHARTER, "saturday-2026", ME);
        // The host's agents are members of the host's rooms.
        board.member(ME, false, ago(72, 0)).member("ada", false, ago(70, 0)).member("site-fixes", true, ago(40, 0)).member("research-notes", true, ago(48, 0));
        // The open task is pushed first so it is `task-1`: the scripted
        // "claim a task" flow through the agent door names it.
        board.push(ago(25, 0), "ada", "task", "Package the photo resizer as a tool", "A container that runs the resizer script as a tool with one `resize` verb. Done = an agent can call it on a folder and get the smaller copies back.", "open", None, Map::new());
        let t1 = board.push(ago(50, 0), ME, "task", "Write a README for the label printer script", "What it does, how to run it, one example. Done = someone new can print a label from it.", "claimed", None, f(&[("claimed_by", json!("ada"))]));
        board.push(ago(30, 0), "ada", "claim", "Write a README for the label printer script", "", "claimed", Some(&t1), Map::new());
        board.push(ago(24, 0), "ada", "offering", "Weekend GPU time on the studio PC", "One card, 24 GB, Saturday and Sunday. Measured in seconds; friends free.", "offered", None, f(&[("pricing", json!("metered")), ("terms", json!("measured, not priced"))]));
        board.push(ago(26, 30), ME, "show", "The desktop app, draft one", "It runs: agents, storage, machines, saved views.", "shown", None, Map::new());
        inner.rooms.insert(board.id.clone(), board);

        let mut idea = Room::new("idea-ada", "A zine for the neighbourhood", Kind::Idea, ada(), false, IDEA_CHARTER, "ideas-open", ME);
        idea.member("ada", false, ago(90, 0)).member(ME, false, ago(80, 0)).member("ada/scout", true, ago(80, 0));
        let d1 = idea.push(ago(70, 0), "ada", "direction", "Recipes from every street", "One page per street, cooked by whoever lives there.", "open", None, f(&[("prefer", json!(1))]));
        let d2 = idea.push(ago(60, 0), ME, "direction", "A map drawn by kids", "Hand-drawn, scanned, the centre spread.", "open", None, f(&[("prefer", json!(2))]));
        idea.push(ago(55, 0), "ada", "steer", "A map drawn by kids", "This is the one. Recipes can be issue two.", "prefer", Some(&d2), Map::new());
        idea.push(ago(54, 0), "ada/scout", "steer", "Recipes from every street", "Recipe zines are common; a map drawn by kids is rarer.", "note", Some(&d1), Map::new());
        idea.push(ago(40, 0), "ada", "synthesis", "Where it stands", "Issue one is the kids' map; recipes wait for issue two.", "open", None, Map::new());
        inner.rooms.insert(idea.id.clone(), idea);
        inner.joined.insert("idea-ada".into());

        let mut dm = Room::new("dm-ada", "ada", Kind::Dm, mine(), true, "Two people. Nothing leaves this room.", "dm", ME);
        dm.member(ME, false, ago(200, 0)).member("ada", false, ago(200, 0));
        dm.push(ago(5, 0), "ada", "say", "", "did the draft run?", "said", None, Map::new());
        dm.push(ago(4, 50), ME, "say", "", "yes — screenshots tomorrow", "said", None, Map::new());
        dm.push(ago(4, 49), "ada", "say", "", "🔥", "said", None, Map::new());
        inner.rooms.insert(dm.id.clone(), dm);

        let mut music = Room::new("music-ren", "Ren's music room", Kind::Home, ren(), false, "Tracks I make. No AI. Say what you hear.", "ren-open", ME);
        music.member("ren", false, ago(100, 0));
        music.push(ago(9, 0), "ren", "show", "New background track for a vid", "Mixed with the lows under 120 Hz kept down so a voice sits over it.", "shown", None, Map::new());
        inner.rooms.insert(music.id.clone(), music);

        inner.pending.push(Knock { knock_id: 1, space: Id { id: "board-saturday".into() }, authorize: Authorize { address: "10.0.0.42".parse::<IpAddr>().unwrap(), authorization: "saturday-2026 as ren".into() }, at: ago(0, 2) });
        inner.next_knock = 2;
    }

    fn author(caller: &Caller) -> String {
        match caller {
            Caller::Person => ME.into(),
            Caller::Agent(name) => name.clone(),
        }
    }

    /// ada answers an ask a few seconds later — a second person in the room.
    fn ada_replies(&self, id: String, ask: String) {
        let this = self.clone();
        self.rt.spawn(async move {
            tokio::time::sleep(Duration::from_secs(4)).await;
            let mut inner = this.lock();
            if let Some(room) = inner.rooms.get_mut(&id) {
                if room.members.iter().any(|m| m.name == "ada") {
                    let params = CallToolRequestParams::new("reply").with_arguments(json!({ "move_id": ask, "body": "I can look at that tomorrow — say a bit more about what done looks like?" }).as_object().cloned().unwrap_or_default());
                    let _ = room.call(params, "ada");
                }
            }
        });
    }
}

#[async_trait]
impl Spaces for StubSpaces {
    async fn list(&self) -> Vec<SpaceEntry> {
        let inner = self.lock();
        inner
            .rooms
            .values()
            .filter(|r| r.mine || inner.joined.contains(&r.id))
            .map(|r| SpaceEntry { id: Id { id: r.id.clone() }, title: r.title.clone(), kind: r.kind.key().into(), host: r.host.clone(), mine: r.mine, online: r.online, joined_as: r.joined_as.clone() })
            .collect()
    }

    async fn host(&self, container: Container) -> Result<Id, WireError> {
        let args = container.arguments;
        let get = |k: &str| args.get(k).and_then(Value::as_str).map(str::to_owned);
        let title = get("title").filter(|t| !t.trim().is_empty()).ok_or_else(|| WireError(json!({ "message": "a Space needs a title" })))?;
        let kind = Kind::parse(&get("kind").unwrap_or_else(|| "home".into())).ok_or_else(|| WireError(json!({ "message": "no such kind of Space" })))?;
        let mut inner = self.lock();
        let id = format!("space-{}", inner.next_room);
        inner.next_room += 1;
        let mut room = Room::new(&id, &title, kind, mine(), true, &get("charter").unwrap_or_default(), &get("invite").unwrap_or_else(|| format!("{id}-key")), ME);
        room.member(ME, false, Utc::now());
        inner.rooms.insert(id.clone(), room);
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

    async fn answer(&self, knock_id: u64, answer: Answer) -> Result<(), String> {
        let mut inner = self.lock();
        let at = inner.pending.iter().position(|k| k.knock_id == knock_id).ok_or("nobody is at that door any more")?;
        let knock = inner.pending.remove(at);
        if answer == Answer::Authorized {
            let name = knock.authorize.authorization.split(" as ").nth(1).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).unwrap_or_else(|| knock.authorize.address.to_string());
            if let Some(room) = inner.rooms.get_mut(&knock.space.id) {
                room.join(&name, false);
            }
        }
        Ok(())
    }

    async fn join(&self, invite: Invite, as_name: String) -> Joined {
        let mut inner = self.lock();
        let Some(room) = inner.rooms.get_mut(&invite.connect.id) else { return Joined::Missing };
        if room.mine {
            return Joined::Error(WireError(json!({ "message": "you host that room" })));
        }
        if room.host != invite.host {
            return Joined::Missing;
        }
        if invite.connect.authorization.split(" as ").next().map(str::trim) != Some(room.invite.as_str()) {
            return Joined::Denied;
        }
        room.joined_as = if as_name.trim().is_empty() { ME.into() } else { as_name };
        room.join(ME, false);
        let id = room.id.clone();
        inner.joined.insert(id.clone());
        Joined::Joined(Id { id })
    }

    async fn leave(&self, id: &Id) -> Result<(), String> {
        let mut inner = self.lock();
        let room = inner.rooms.get_mut(&id.id).ok_or("no such room")?;
        if room.mine {
            inner.rooms.shift_remove(&id.id);
        } else {
            room.members.retain(|m| m.name != ME);
            inner.joined.remove(&id.id);
        }
        Ok(())
    }

    async fn tools(&self, id: &Id) -> Result<ListToolsResult, ErrorData> {
        let inner = self.lock();
        inner.rooms.get(&id.id).map(Room::tools).ok_or_else(|| ErrorData::invalid_request("no such room", None))
    }

    async fn read(&self, id: &Id, uri: &str) -> Result<ReadResourceResult, ErrorData> {
        let inner = self.lock();
        inner.rooms.get(&id.id).ok_or_else(|| ErrorData::invalid_request("no such room", None))?.read(uri)
    }

    async fn call(&self, id: &Id, params: CallToolRequestParams, caller: Caller) -> Result<CallToolResult, ErrorData> {
        let author = Self::author(&caller);
        let tool = params.name.to_string();
        let result = {
            let mut inner = self.lock();
            let room = inner.rooms.get_mut(&id.id).ok_or_else(|| ErrorData::invalid_request("no such room", None))?;
            if !room.online {
                return Err(ErrorData::internal_error("the room's host is offline", None));
            }
            room.call(params, &author)?
        };
        if tool == "ask" && author != "ada" {
            let ask_id = self.lock().rooms.get(&id.id).and_then(|r| r.moves.last().map(|m| m.id.clone()));
            if let Some(ask_id) = ask_id {
                self.ada_replies(id.id.clone(), ask_id);
            }
        }
        Ok(result)
    }

    fn notifications(&self, id: &Id, cancel: CancellationToken) -> Frames<ServerNotification> {
        let (tx, rx) = mpsc::channel::<ServerNotification>(64);
        let Some(mut live) = self.lock().rooms.get(&id.id).map(|r| r.live.subscribe()) else {
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

    async fn home(&self) -> Option<Id> {
        let inner = self.lock();
        inner.rooms.values().find(|r| r.mine && r.kind == Kind::Home).map(|r| Id { id: r.id.clone() })
    }

    async fn invite(&self, id: &Id) -> Option<Invite> {
        let inner = self.lock();
        let room = inner.rooms.get(&id.id)?;
        if !room.mine {
            return None;
        }
        Some(Invite { host: mine(), connect: Connect { id: room.id.clone(), authorization: room.invite.clone() } })
    }
}

const HOME_CHARTER: &str = "# Your home\n\nWhat you're building, shown when you choose to. Friends may ask and answer. Your agents post what they finished.";
const BOARD_CHARTER: &str = "# Saturday Workshop\n\n## Rules\nShow what you're making, finished or not. Ask for help plainly. Be kind about other people's work.\n\n## Who does what\nThe **host** keeps the rules and accepts deliveries. **Members** post, claim and offer. **Guests** read.\n\n## Tasks\nA task has a title and a spec, fixed when it's posted. Claim it as posted; the spec is what gets checked.\n\n## Receipts\nAccepting a delivery issues a receipt to whoever did the work. Receipts stay in this room and are never a rating of a person.";
const IDEA_CHARTER: &str = "# A zine for the neighbourhood\n\nPropose directions, steer them (prefer, reject, note), and write where it stands. Each round starts from the last synthesis.";

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use rmcp::model::ResourceContents;

    fn feed(spaces: &StubSpaces, id: &str) -> Vec<rooms::Move> {
        let inner = spaces.lock();
        inner.rooms.get(id).map(|r| r.moves.clone()).unwrap_or_default()
    }

    fn call(name: &str, args: Value) -> CallToolRequestParams {
        CallToolRequestParams::new(name.to_owned()).with_arguments(args.as_object().cloned().unwrap_or_default())
    }

    #[tokio::test]
    async fn the_door_lets_someone_in_only_when_the_host_says_so() {
        let spaces = StubSpaces::new();
        let cancel = CancellationToken::new();
        let knock = spaces.knocks(cancel.clone()).next().await.expect("someone is at the door");
        cancel.cancel();
        assert_eq!(knock.space.id, "board-saturday");
        assert_eq!(knock.authorize.authorization, "saturday-2026 as ren");
        spaces.answer(knock.knock_id, Answer::Authorized).await.unwrap();
        let members = spaces.read(&Id { id: "board-saturday".into() }, rooms::MEMBERS).await.unwrap();
        let ResourceContents::TextResourceContents { text, .. } = &members.contents[0] else { panic!() };
        assert!(text.contains("\"ren\""), "ren was let in: {text}");
        assert!(spaces.answer(knock.knock_id, Answer::Denied).await.is_err(), "answered twice");
    }

    #[tokio::test]
    async fn joining_needs_the_right_invite() {
        let spaces = StubSpaces::new();
        let host = ren();
        let wrong = Invite { host: host.clone(), connect: Connect { id: "music-ren".into(), authorization: "nope as me".into() } };
        assert!(matches!(spaces.join(wrong, "maya".into()).await, Joined::Denied));
        let missing = Invite { host: host.clone(), connect: Connect { id: "no-such-room".into(), authorization: "ren-open".into() } };
        assert!(matches!(spaces.join(missing, "maya".into()).await, Joined::Missing));
        let right = Invite { host, connect: Connect { id: "music-ren".into(), authorization: "ren-open as maya".into() } };
        assert!(matches!(spaces.join(right, "maya".into()).await, Joined::Joined(_)));
        assert!(spaces.list().await.iter().any(|e| e.id.id == "music-ren"), "joined rooms are listed");
        // The invite a host hands out round-trips through the text form.
        let mine = spaces.invite(&Id { id: "board-saturday".into() }).await.unwrap();
        assert_eq!(mine.connect.authorization, "saturday-2026");
    }

    #[tokio::test]
    async fn a_task_goes_post_claim_deliver_accept_receipt() {
        let spaces = StubSpaces::new();
        let board = Id { id: "board-saturday".into() };
        let cancel = CancellationToken::new();
        let mut heard = spaces.notifications(&board, cancel.clone());
        spaces.call(&board, call("post_task", json!({ "title": "Name the tabs", "spec": "Six words, tested on the six use cases." })), Caller::Person).await.unwrap();
        assert!(matches!(heard.next().await, Some(ServerNotification::ResourceUpdatedNotification(_))), "watchers hear the change");
        let task = feed(&spaces, "board-saturday").into_iter().find(|m| m.title == "Name the tabs").unwrap();
        assert_eq!(task.author, ME);
        let agent = Caller::Agent("site-fixes".into());
        spaces.call(&board, call("claim", json!({ "task_id": task.id })), agent.clone()).await.unwrap();
        assert!(spaces.call(&board, call("claim", json!({ "task_id": task.id })), Caller::Person).await.is_err(), "claimed twice");
        assert!(spaces.call(&board, call("accept", json!({ "task_id": task.id })), Caller::Person).await.is_err(), "nothing delivered yet");
        spaces.call(&board, call("deliver", json!({ "task_id": task.id, "summary": "Six words." })), agent).await.unwrap();
        spaces.call(&board, call("accept", json!({ "task_id": task.id })), Caller::Person).await.unwrap();
        let moves = feed(&spaces, "board-saturday");
        assert_eq!(moves.iter().find(|m| m.id == task.id).unwrap().state, "done");
        let receipt = moves.iter().find(|m| m.kind == "receipt" && m.parent.as_deref() == Some(&task.id)).expect("a receipt was issued");
        assert_eq!(receipt.fields.get("to").and_then(Value::as_str), Some("site-fixes"));
        assert_eq!(receipt.title, "Name the tabs");
        cancel.cancel();
    }

    #[tokio::test]
    async fn only_members_may_act_and_rooms_have_their_own_verbs() {
        let spaces = StubSpaces::new();
        let home = Id { id: "home-me".into() };
        assert!(spaces.call(&home, call("show", json!({ "title": "x" })), Caller::Agent("stranger".into())).await.is_err());
        assert!(spaces.call(&home, call("post_task", json!({ "title": "x", "spec": "y" })), Caller::Person).await.is_err(), "a home has no work board");
        let dm = spaces.tools(&Id { id: "dm-ada".into() }).await.unwrap();
        assert_eq!(dm.tools.iter().map(|t| t.name.to_string()).collect::<Vec<_>>(), vec!["say", "reply"]);
        let hosted = spaces.host(Container { image: diverge_sdk::shared::containers::request::Image { name: "diverge-space-idea".into(), digest: "unbuilt".into() }, memory: 1, disk: 1, volume_mounts: vec![], fuse_file_mounts: vec![], fuse_directory_mounts: vec![], arguments: json!({ "title": "Naming", "kind": "idea", "charter": "", "invite": "names" }) }).await.unwrap();
        assert!(spaces.tools(&hosted).await.unwrap().tools.iter().any(|t| t.name == "propose"));
        spaces.leave(&hosted).await.unwrap();
        assert!(spaces.tools(&hosted).await.is_err(), "an ended room is gone");
    }
}
