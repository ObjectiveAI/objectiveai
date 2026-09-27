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
//! it is. And it asks first: a card before each move in a room, unless its
//! person set an allowance there. Reports to your own Home need no asking.

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
use crate::view::{CardEvent, CardKind, CardView};

struct Pending {
    view: CardView,
    answer: oneshot::Sender<String>,
}

/// How many moves an agent may make in a room each day without asking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Allowance {
    pub per_day: u32,
    pub used: u32,
    pub day: Option<NaiveDate>,
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
            a.used = 0;
        }
        a
    }

    pub fn set_allowance(&self, room: &str, agent: &str, per_day: u32) {
        let mut all = self.allowances.lock().unwrap();
        let a = all.entry(Self::slot(room, agent)).or_default();
        a.per_day = per_day;
        if let Some(f) = &self.allowances_file {
            let _ = std::fs::write(f, serde_json::to_string_pretty(&*all).unwrap_or_default());
        }
    }

    /// Spend one move of an agent's allowance in a room, if it has one left today.
    fn spend(&self, room: &str, agent: &str) -> bool {
        let today = Utc::now().date_naive();
        let mut all = self.allowances.lock().unwrap();
        let Some(a) = all.get_mut(&Self::slot(room, agent)) else { return false };
        if a.day != Some(today) {
            a.day = Some(today);
            a.used = 0;
        }
        if a.used < a.per_day {
            a.used += 1;
            if let Some(f) = &self.allowances_file {
                let _ = std::fs::write(f, serde_json::to_string_pretty(&*all).unwrap_or_default());
            }
            true
        } else {
            false
        }
    }

    /// Whether an agent may make this move in this room: its allowance, or its person's yes.
    async fn may(&self, agent: &str, id: &Id, verb: &str, args: &JsonObject) -> bool {
        if verb == "report" && self.spaces.home().await.as_ref() == Some(id) {
            return true;
        }
        if self.spend(&id.id, agent) {
            return true;
        }
        let room = self.spaces.list().await.into_iter().find(|e| &e.id == id).map(|e| e.title).unwrap_or_else(|| id.id.clone());
        let named = |key: &str| args.get(key).and_then(Value::as_str).map(str::to_owned);
        let about = match verb {
            "claim" | "deliver" | "settle" => {
                let task = named("task_id").unwrap_or_default();
                let title = self.title_of(id, &task).await.unwrap_or(task);
                match verb {
                    "claim" => format!("Claim “{title}” in {room}? Claiming commits you to its spec."),
                    "deliver" => format!("Deliver on “{title}” in {room}?"),
                    _ => format!("Say whether “{title}” is settled, in {room}?"),
                }
            }
            "show" => format!("Show “{}” in {room}?", named("title").unwrap_or_default()),
            "ask" => format!("Ask “{}” in {room}?", named("what").unwrap_or_default()),
            "offer" => format!("Offer to serve an ask in {room}: “{}”?", named("body").unwrap_or_default()),
            "reply" | "say" | "leave_note" => format!("Say this in {room}: “{}”?", named("body").unwrap_or_default()),
            "table_write" => format!("Put {} on the table in {room}?", named("path").unwrap_or_default()),
            other => format!("{other} in {room}?"),
        };
        self.ask(agent, about, CardKind::Choice, vec!["Yes".into(), "No".into()]).await == "Yes"
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

    /// Put a card in front of the person and wait for the answer: for the
    /// app's own questions too, like a hire through the profile.
    /// A card about one of your agents that someone else is asking, through
    /// a room of yours: a visitor's hire. Nothing runs until you answer.
    pub async fn ask_for(&self, agent: &str, from: &str, question: String, kind: CardKind, options: Vec<String>) -> String {
        self.card(agent, Some(from.to_owned()), question, kind, options).await
    }

    async fn ask(&self, agent: &str, question: String, kind: CardKind, options: Vec<String>) -> String {
        self.card(agent, None, question, kind, options).await
    }

    async fn card(&self, agent: &str, from: Option<String>, question: String, kind: CardKind, options: Vec<String>) -> String {
        let (tx, rx) = oneshot::channel();
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let view = CardView { id, agent: agent.to_owned(), from, kind, question, options, at: Utc::now().to_rfc3339() };
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
                if !self.may(agent, &id, &tool, &call_args).await {
                    return Ok(CallToolResult::success(vec![ContentBlock::text("Your person said no. Nothing was done.")]));
                }
                let mut inner = CallToolRequestParams::new(tool).with_arguments(call_args);
                self.identity.seal(&Actor::Agent(agent.to_owned()), &id.id, &mut inner).map_err(|e| ErrorData::internal_error(e, None))?;
                let result = self.spaces.call(&id, inner).await?;
                result.content.iter().filter_map(|c| c.as_text().map(|t| t.text.clone())).collect::<Vec<_>>().join("\n")
            }
            "asks_open" => {
                let mut open = Vec::new();
                for e in self.spaces.list().await {
                    let (key, _) = self.identity.agent_in(agent, Some(&e.id.id));
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
                let parts: Vec<String> = path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect();
                let bytes = self.spaces.table_read(&id, &parts).await.map_err(|e| ErrorData::internal_error(crate::view::error_text(&e), None))?;
                String::from_utf8_lossy(&bytes).into_owned()
            }
            "table_write" => {
                let id = Id { id: get("space").ok_or_else(|| ErrorData::invalid_params("space is needed", None))? };
                let path = get("path").ok_or_else(|| ErrorData::invalid_params("path is needed", None))?;
                if !self.may(agent, &id, "table_write", &args).await {
                    return Ok(CallToolResult::success(vec![ContentBlock::text("Your person said no. Nothing was put on the table.")]));
                }
                let parts: Vec<String> = path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect();
                self.spaces.table_write(&id, &parts, get("text").unwrap_or_default().into_bytes()).await.map_err(|e| ErrorData::internal_error(crate::view::error_text(&e), None))?;
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
