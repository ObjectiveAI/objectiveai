//! The agent door: the MCP server this app is, for its person's agents.
//!
//! On the wire an agent container reaches "the MCP servers of the caller"
//! through the run's `mcp-list-tools` / `mcp-call-tool` channels, which
//! the daemon relays to the app. What the app answers is here: the same
//! Space verbs a person clicks, and `ask_person` — a question, a choice
//! or a credential-by-meaning, put in front of the person as a card. The
//! agent waits for the answer; nothing times out. A credential's value
//! never reaches the agent: it learns that a key was used, not which.
//!
//! Everything an agent does in a room is sealed with its own key, tethered
//! to its person (see [`crate::identity`]), so the room knows whose agent
//! it is. And it asks first: a card before each move in a room, and before
//! reading one, unless its person set an allowance there for that kind of
//! move. The allowance is the person's own setting, never the app's rule.
//! Reports to your Home need no asking only while Home is yours alone.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use futures::stream;
use rmcp::model::{CallToolRequestParams, CallToolResult, ContentBlock, ErrorData, JsonObject, ListToolsResult, Tool};
use serde_json::{Value, json};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::daemon::Frames;
use crate::identity::{Actor, Identity};
use crate::spaces::{Id, Spaces};
use crate::view::{CardCall, CardEvent, CardHire, CardKind, CardView};

struct Pending {
    view: CardView,
    answer: oneshot::Sender<String>,
}

/// What kind of move something is, for an allowance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum Reach {
    /// Reading a room: what's happened, its table.
    Read,
    /// Saying something: show, ask, offer, reply.
    Talk,
    /// Taking on or handing in work: claim, deliver, a file on the table.
    Work,
    /// Anything with a pledge in it, or saying one is settled.
    Pledge,
}

impl Reach {
    pub fn of(verb: &str, args: &JsonObject) -> Reach {
        let pledged = args.get("pledge").and_then(Value::as_str).is_some_and(|p| !p.trim().is_empty());
        match verb {
            _ if pledged => Reach::Pledge,
            "settle" | "hire" => Reach::Pledge,
            "space_feed" | "table_list" | "table_read" | "asks_open" => Reach::Read,
            "show" | "ask" | "offer" | "reply" | "say" | "leave_note" | "report" | "propose" | "steer" | "synthesize" => Reach::Talk,
            _ => Reach::Work,
        }
    }
}

/// How many moves of each kind an agent may make in a room each day without asking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Allowance {
    #[serde(default)]
    pub per_day: HashMap<Reach, u32>,
    #[serde(default)]
    pub used: HashMap<Reach, u32>,
    pub day: Option<NaiveDate>,
}

/// Whether a move may go ahead, and if on an allowance, which to give back if the room refuses it.
enum Permit {
    Free,
    Allowed(Reach),
    Yes,
    No,
}

pub struct Door {
    spaces: Arc<dyn Spaces>,
    identity: Arc<Identity>,
    /// (room, agent) → allowance. Yours, kept by the app.
    allowances: Mutex<HashMap<String, Allowance>>,
    allowances_file: Option<PathBuf>,
    cards: Mutex<Vec<Pending>>,
    live: broadcast::Sender<CardEvent>,
    next: AtomicU64,
    rt: tokio::runtime::Handle,
}

/// A card's two answers for a move. The screen words them; these are what comes back.
pub const YES: &str = "yes";
pub const NO: &str = "no";

/// The stand-in vault: names only. The values are the daemon's, never the app's.
const VAULT: &[&str] = &["OpenRouter key", "Calendar key", "GitHub token", "Anthropic key"];

fn schema(props: Value, required: &[&str]) -> Arc<JsonObject> {
    let mut o = JsonObject::new();
    o.insert("type".into(), json!("object"));
    o.insert("properties".into(), props);
    o.insert("required".into(), json!(required));
    Arc::new(o)
}

impl Door {
    pub fn new(spaces: Arc<dyn Spaces>, identity: Arc<Identity>, allowances_file: Option<PathBuf>) -> Self {
        let (live, _) = broadcast::channel(64);
        let allowances = allowances_file.as_ref().and_then(|f| std::fs::read_to_string(f).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Door {
            spaces,
            identity,
            allowances: Mutex::new(allowances),
            allowances_file,
            cards: Mutex::new(Vec::new()),
            live,
            next: AtomicU64::new(1),
            rt: tokio::runtime::Handle::current(),
        }
    }

    fn slot(room: &str, agent: &str) -> String {
        format!("{room}\u{1f}{agent}")
    }

    pub fn allowance(&self, room: &str, agent: &str) -> Allowance {
        let mut a = self.allowances.lock().unwrap().get(&Self::slot(room, agent)).cloned().unwrap_or_default();
        if a.day != Some(Utc::now().date_naive()) {
            a.used.clear();
        }
        a
    }

    fn save_allowances(&self, all: &HashMap<String, Allowance>) {
        if let Some(f) = &self.allowances_file {
            let _ = std::fs::write(f, serde_json::to_string_pretty(all).unwrap_or_default());
        }
    }

    pub fn set_allowance(&self, room: &str, agent: &str, reach: Reach, per_day: u32) {
        let mut all = self.allowances.lock().unwrap();
        all.entry(Self::slot(room, agent)).or_default().per_day.insert(reach, per_day);
        self.save_allowances(&all);
    }

    /// Take one move of this kind from an agent's allowance in a room, if it has one left today.
    fn reserve(&self, room: &str, agent: &str, reach: Reach) -> bool {
        let today = Utc::now().date_naive();
        let mut all = self.allowances.lock().unwrap();
        let Some(a) = all.get_mut(&Self::slot(room, agent)) else { return false };
        if a.day != Some(today) {
            a.day = Some(today);
            a.used.clear();
        }
        let used = a.used.get(&reach).copied().unwrap_or(0);
        if used < a.per_day.get(&reach).copied().unwrap_or(0) {
            a.used.insert(reach, used + 1);
            self.save_allowances(&all);
            true
        } else {
            false
        }
    }

    /// Give back a move the room refused: an allowance is spent only on what happened.
    fn refund(&self, room: &str, agent: &str, reach: Reach) {
        let mut all = self.allowances.lock().unwrap();
        if let Some(n) = all.get_mut(&Self::slot(room, agent)).and_then(|a| a.used.get_mut(&reach)) {
            *n = n.saturating_sub(1);
        }
        self.save_allowances(&all);
    }

    /// Whether your Home has anyone in it besides you and your agents.
    async fn home_is_yours_alone(&self, home: &Id) -> bool {
        let Ok(r) = self.spaces.read(home, diverge_desktop_room::room::MEMBERS).await else { return false };
        let Some(text) = r.contents.iter().find_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => Some(text.clone()), _ => None }) else { return false };
        let members: Vec<Value> = serde_json::from_str(&text).unwrap_or_default();
        members.iter().all(|m| m["key"].as_str().is_some_and(|k| self.identity.owner_of(k).is_some()))
    }

    /// Whether an agent may make this move in this room: free (a report to a
    /// Home that's yours alone), its allowance for this kind of move, or its
    /// person's yes on a card showing the whole call.
    async fn may(&self, agent: &str, id: &Id, verb: &str, args: &JsonObject) -> Permit {
        if verb == "report" && self.spaces.home().await.as_ref() == Some(id) && self.home_is_yours_alone(id).await {
            return Permit::Free;
        }
        let reach = Reach::of(verb, args);
        if self.reserve(&id.id, agent, reach) {
            return Permit::Allowed(reach);
        }
        let room_title = self.spaces.list().await.into_iter().find(|e| &e.id == id).map(|e| e.title).unwrap_or_else(|| id.id.clone());
        let named = ["task_id", "ask_id", "move_id", "direction_id", "hire_id"].iter().find_map(|k| args.get(*k).and_then(Value::as_str).map(str::to_owned));
        let about = match named {
            Some(m) => self.title_of(id, &m).await,
            None => None,
        };
        let call = CardCall { room: id.id.clone(), room_title, verb: verb.to_owned(), reach, arguments: Value::Object(args.clone()), about };
        if self.card(agent, None, String::new(), CardKind::Choice, vec![YES.into(), NO.into()], Some(call), None).await == YES { Permit::Yes } else { Permit::No }
    }

    async fn title_of(&self, id: &Id, move_id: &str) -> Option<String> {
        let r = self.spaces.read(id, diverge_desktop_room::room::FEED).await.ok()?;
        let text = r.contents.iter().find_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => Some(text.clone()), _ => None })?;
        let moves: Vec<Value> = serde_json::from_str(&text).ok()?;
        moves.iter().find(|m| m["id"] == move_id).and_then(|m| m["title"].as_str()).map(str::to_owned)
    }

    pub fn tools(&self) -> ListToolsResult {
        ListToolsResult::with_all_items(vec![
            Tool::new("spaces_list", "The Spaces your person hosts or has joined.", schema(json!({}), &[])),
            Tool::new("space_feed", "What has happened in a Space: its moves, newest last.", schema(json!({ "space": { "type": "string" } }), &["space"])),
            Tool::new("space_tools", "A Space's own verbs, with their arguments.", schema(json!({ "space": { "type": "string" } }), &["space"])),
            Tool::new("space_call", "Do something in a Space: one of its verbs, with arguments. You act as yourself, sealed as your person's agent; your person is asked first unless they allowed it.", schema(json!({ "space": { "type": "string" }, "tool": { "type": "string" }, "arguments": { "type": "object" } }), &["space", "tool"])),
            Tool::new("asks_open", "Open asks in the Spaces you're in: what people and agents need, where, and who may serve it. Offer with space_call's `offer`.", schema(json!({}), &[])),
            Tool::new("table_list", "The files on a Space's table: everyone in the room sees the same ones.", schema(json!({ "space": { "type": "string" } }), &["space"])),
            Tool::new("table_read", "Read one file off a Space's table.", schema(json!({ "space": { "type": "string" }, "path": { "type": "string" } }), &["space", "path"])),
            Tool::new("table_write", "Put a text file on a Space's table. Everyone in the room can see and change it.", schema(json!({ "space": { "type": "string" }, "path": { "type": "string" }, "text": { "type": "string" } }), &["space", "path", "text"])),
            Tool::new(
                "ask_person",
                "Ask your person something and wait for the answer. A question (free text), a choice (options), or a credential described by meaning — you get told a key was used, never its value.",
                schema(
                    json!({
                        "question": { "type": "string" },
                        "kind": { "type": "string", "enum": ["question", "choice", "credential"] },
                        "options": { "type": "array", "items": { "type": "string" } }
                    }),
                    &["question"],
                ),
            ),
        ])
    }

    pub fn cards(&self) -> Vec<CardView> {
        self.cards.lock().unwrap().iter().map(|p| p.view.clone()).collect()
    }

    pub fn watch(&self, cancel: CancellationToken) -> Frames<CardEvent> {
        let (tx, rx) = mpsc::channel(64);
        let pending: Vec<CardEvent> = self.cards().into_iter().map(|card| CardEvent::Card { card }).collect();
        let mut live = self.live.subscribe();
        self.rt.spawn(async move {
            for e in pending {
                if tx.send(e).await.is_err() {
                    return;
                }
            }
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    next = live.recv() => match next {
                        Ok(e) => { if tx.send(e).await.is_err() { return; } }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => return,
                    },
                }
            }
        });
        Box::pin(stream::unfold(rx, |mut rx| async move { rx.recv().await.map(|e| (e, rx)) }))
    }

    pub fn answer(&self, id: u64, answer: String) -> Result<(), String> {
        let mut cards = self.cards.lock().unwrap();
        let at = cards.iter().position(|p| p.view.id == id).ok_or("that card was already answered")?;
        let p = cards.remove(at);
        let _ = p.answer.send(answer);
        let _ = self.live.send(CardEvent::Answered { id });
        Ok(())
    }

    /// A card about one of your agents that someone else is asking for,
    /// through a room of yours: a visitor's hire. Nothing runs until you answer.
    pub async fn ask_hire(&self, agent: &str, hire: CardHire, options: Vec<String>) -> String {
        self.card(agent, Some(hire.from.clone()), String::new(), CardKind::Choice, options, None, Some(hire)).await
    }

    async fn ask(&self, agent: &str, question: String, kind: CardKind, options: Vec<String>) -> String {
        self.card(agent, None, question, kind, options, None, None).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn card(&self, agent: &str, from: Option<String>, question: String, kind: CardKind, options: Vec<String>, call: Option<CardCall>, hire: Option<CardHire>) -> String {
        let (tx, rx) = oneshot::channel();
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let view = CardView { id, agent: agent.to_owned(), from, kind, question, options, at: Utc::now().to_rfc3339(), call, hire };
        self.cards.lock().unwrap().push(Pending { view: view.clone(), answer: tx });
        let _ = self.live.send(CardEvent::Card { card: view });
        rx.await.unwrap_or_default()
    }

    /// What the wire's `mcp-call-tool` hands us for one of this person's agents.
    pub async fn call(&self, agent: &str, params: CallToolRequestParams) -> Result<CallToolResult, ErrorData> {
        let args = params.arguments.unwrap_or_default();
        let get = |k: &str| args.get(k).and_then(Value::as_str).map(str::to_owned);
        let text = match params.name.as_ref() {
            "spaces_list" => {
                let list = self.spaces.list().await;
                serde_json::to_string(&list.iter().map(|e| json!({ "id": e.id.id, "title": e.title, "kind": e.kind, "online": e.online })).collect::<Vec<_>>()).unwrap_or_default()
            }
            "space_feed" => {
                let id = Id { id: get("space").ok_or_else(|| ErrorData::invalid_params("space is needed", None))? };
                if matches!(self.may(agent, &id, "space_feed", &args).await, Permit::No) {
                    return Ok(CallToolResult::success(vec![ContentBlock::text("Your person said no. Nothing was read.")]));
                }
                let r = self.spaces.read(&id, diverge_desktop_room::room::FEED).await?;
                r.contents.iter().filter_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => Some(text.clone()), _ => None }).next().unwrap_or_default()
            }
            "space_tools" => {
                let id = Id { id: get("space").ok_or_else(|| ErrorData::invalid_params("space is needed", None))? };
                serde_json::to_string(&self.spaces.tools(&id).await?.tools).unwrap_or_default()
            }
            "space_call" => {
                let id = Id { id: get("space").ok_or_else(|| ErrorData::invalid_params("space is needed", None))? };
                let tool = get("tool").ok_or_else(|| ErrorData::invalid_params("tool is needed", None))?;
                let call_args = args.get("arguments").and_then(Value::as_object).cloned().unwrap_or_default();
                let permit = self.may(agent, &id, &tool, &call_args).await;
                if matches!(permit, Permit::No) {
                    return Ok(CallToolResult::success(vec![ContentBlock::text("Your person said no. Nothing was done.")]));
                }
                let mut inner = CallToolRequestParams::new(tool).with_arguments(call_args);
                let actor = Actor::Agent(agent.to_owned());
                let result = {
                    let turn = self.identity.turn(&actor, &id.id);
                    let _held = turn.lock().await;
                    match self.identity.seal(&actor, &id.id, &mut inner) {
                        Ok(_) => self.spaces.call(&id, inner).await,
                        Err(e) => Err(ErrorData::internal_error(e, None)),
                    }
                };
                let refused = !matches!(&result, Ok(r) if r.is_error != Some(true));
                if let (Permit::Allowed(reach), true) = (&permit, refused) {
                    self.refund(&id.id, agent, *reach);
                }
                let result = result?;
                result.content.iter().filter_map(|c| c.as_text().map(|t| t.text.clone())).collect::<Vec<_>>().join("\n")
            }
            "asks_open" => {
                // Reading every room at once: rooms it may read on its allowance, and the rest only on one yes.
                let entries = self.spaces.list().await;
                let (allowed, rest): (Vec<_>, Vec<_>) = entries.into_iter().partition(|e| self.reserve(&e.id.id, agent, Reach::Read));
                let mut readable = allowed;
                if !rest.is_empty() {
                    let titles = rest.iter().map(|e| e.title.clone()).collect::<Vec<_>>().join(", ");
                    let call = CardCall { room: String::new(), room_title: titles, verb: "asks_open".into(), reach: Reach::Read, arguments: json!({}), about: None };
                    if self.card(agent, None, String::new(), CardKind::Choice, vec![YES.into(), NO.into()], Some(call), None).await == YES {
                        readable.extend(rest);
                    }
                }
                let mut open = Vec::new();
                for e in readable {
                    let key = self.identity.agent_in(agent, Some(&e.id.id)).key;
                    let Ok(r) = self.spaces.read(&e.id, diverge_desktop_room::room::FEED).await else { continue };
                    let Some(text) = r.contents.iter().find_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => Some(text.clone()), _ => None }) else { continue };
                    let Ok(moves) = serde_json::from_str::<Vec<Value>>(&text) else { continue };
                    let person = self.identity.in_room(&e.id.id).map(|p| p.key).unwrap_or_else(|| self.identity.usual().key);
                    for m in moves.iter().filter(|m| m["kind"] == "ask" && m["state"] == "open") {
                        // Not your own person's asks, and not rooms you aren't in as this agent.
                        if m["by"] == person.as_str() || m["by"] == key.as_str() {
                            continue;
                        }
                        open.push(json!({ "space": e.id.id, "space_title": e.title, "ask_id": m["id"], "what": m["title"], "by": m["author"], "needs": m["fields"]["needs"], "ceiling": m["fields"]["ceiling"], "who_may_serve": m["fields"]["who_may_serve"], "offers": m["fields"]["offers"] }));
                    }
                }
                serde_json::to_string(&open).unwrap_or_default()
            }
            "table_list" => {
                let id = Id { id: get("space").ok_or_else(|| ErrorData::invalid_params("space is needed", None))? };
                if matches!(self.may(agent, &id, "table_list", &args).await, Permit::No) {
                    return Ok(CallToolResult::success(vec![ContentBlock::text("Your person said no. Nothing was read.")]));
                }
                let nodes = self.spaces.table_tree(&id).await.map_err(|e| ErrorData::internal_error(crate::view::error_text(&e), None))?;
                let mut paths = Vec::new();
                fn walk(nodes: &[diverge_sdk::shared::filetree::response::Node], prefix: &str, out: &mut Vec<String>) {
                    use diverge_sdk::shared::filetree::response::Node;
                    for n in nodes {
                        match n {
                            Node::File { name, .. } | Node::Symlink { name, .. } => out.push(format!("{prefix}{name}")),
                            Node::Directory { name, children, .. } => walk(children, &format!("{prefix}{name}/"), out),
                        }
                    }
                }
                walk(&nodes, "", &mut paths);
                paths.join("\n")
            }
            "table_read" => {
                let id = Id { id: get("space").ok_or_else(|| ErrorData::invalid_params("space is needed", None))? };
                let path = get("path").ok_or_else(|| ErrorData::invalid_params("path is needed", None))?;
                if matches!(self.may(agent, &id, "table_read", &args).await, Permit::No) {
                    return Ok(CallToolResult::success(vec![ContentBlock::text("Your person said no. Nothing was read.")]));
                }
                let parts: Vec<String> = path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect();
                let bytes = self.spaces.table_read(&id, &parts).await.map_err(|e| ErrorData::internal_error(crate::view::error_text(&e), None))?;
                String::from_utf8_lossy(&bytes).into_owned()
            }
            "table_write" => {
                let id = Id { id: get("space").ok_or_else(|| ErrorData::invalid_params("space is needed", None))? };
                let path = get("path").ok_or_else(|| ErrorData::invalid_params("path is needed", None))?;
                let permit = self.may(agent, &id, "table_write", &args).await;
                if matches!(permit, Permit::No) {
                    return Ok(CallToolResult::success(vec![ContentBlock::text("Your person said no. Nothing was put on the table.")]));
                }
                let parts: Vec<String> = path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect();
                if let Err(e) = self.spaces.table_write(&id, &parts, get("text").unwrap_or_default().into_bytes()).await {
                    if let Permit::Allowed(reach) = permit {
                        self.refund(&id.id, agent, reach);
                    }
                    return Err(ErrorData::internal_error(crate::view::error_text(&e), None));
                }
                format!("On the table: {path}")
            }
            "ask_person" => {
                let question = get("question").ok_or_else(|| ErrorData::invalid_params("question is needed", None))?;
                let kind = match get("kind").as_deref() {
                    Some("choice") => CardKind::Choice,
                    Some("credential") => CardKind::Credential,
                    _ => CardKind::Question,
                };
                let options = match kind {
                    CardKind::Credential => {
                        let q = question.to_lowercase();
                        let matches: Vec<String> = VAULT.iter().filter(|k| q.split_whitespace().any(|w| w.len() > 3 && k.to_lowercase().contains(w))).map(|k| (*k).to_owned()).collect();
                        if matches.is_empty() { VAULT.iter().map(|k| (*k).to_owned()).collect() } else { matches }
                    }
                    _ => args.get("options").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_default(),
                };
                let answer = self.ask(agent, question, kind.clone(), options).await;
                match kind {
                    CardKind::Credential => format!("A key was used: {answer}. Its value never left the vault."),
                    _ => answer,
                }
            }
            other => return Err(ErrorData::invalid_params(format!("the door has no tool called {other}"), None)),
        };
        Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spaces::stub::{StubSpaces, board};

    fn door() -> (Arc<Door>, StubSpaces) {
        let identity = Arc::new(Identity::stand_in("maya"));
        let tables = std::env::temp_dir().join(format!("diverge-desktop-test-door-{}-{}", std::process::id(), Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        let stub = StubSpaces::new(identity.clone(), tables);
        (Arc::new(Door::new(Arc::new(stub.clone()), identity, None)), stub)
    }

    fn space_call(space: &str, tool: &str, arguments: Value) -> CallToolRequestParams {
        CallToolRequestParams::new("space_call").with_arguments(json!({ "space": space, "tool": tool, "arguments": arguments }).as_object().cloned().unwrap())
    }

    #[tokio::test]
    async fn an_allowance_is_spent_only_on_what_the_room_accepts() {
        let (door, _) = door();
        let board = board();
        door.set_allowance(&board, "research-notes", Reach::Talk, 1);
        let refused = door.call("research-notes", space_call(&board, "offer", json!({ "ask_id": "no-such-ask", "body": "I can" }))).await;
        assert!(refused.is_err() || refused.unwrap().is_error == Some(true), "the room refuses it");
        assert_eq!(door.allowance(&board, "research-notes").used.get(&Reach::Talk).copied().unwrap_or(0), 0, "and it costs nothing");
        door.call("research-notes", space_call(&board, "show", json!({ "title": "a sketch" }))).await.unwrap();
        assert_eq!(door.allowance(&board, "research-notes").used.get(&Reach::Talk), Some(&1));
        assert!(door.cards().is_empty(), "no card while the allowance lasts");
        // Reading isn't talking: a different allowance, so it asks.
        let d = door.clone();
        let b = board.clone();
        tokio::spawn(async move { d.call("research-notes", CallToolRequestParams::new("space_feed").with_arguments(json!({ "space": b }).as_object().cloned().unwrap())).await });
        for _ in 0..50 {
            tokio::task::yield_now().await;
        }
        assert_eq!(door.cards().first().and_then(|c| c.call.as_ref()).map(|c| c.reach), Some(Reach::Read));
    }

    #[tokio::test]
    async fn a_report_to_a_home_others_are_in_asks_first() {
        let (door, stub) = door();
        let home = stub.id_of("home-me");
        let d = door.clone();
        tokio::spawn(async move { d.call("site-fixes", space_call(&home, "report", json!({ "title": "done", "body": "the private details" }))).await });
        for _ in 0..50 {
            tokio::task::yield_now().await;
        }
        let card = door.cards().into_iter().next().expect("ada is in your Home, so it asks");
        let call = card.call.expect("the whole call");
        assert_eq!(call.verb, "report");
        assert_eq!(call.arguments["body"], "the private details", "and the card shows what would be said");
    }
}
