//! The agent door: the MCP server this app is, for its person's agents.
//!
//! On the wire an agent container reaches "the MCP servers of the caller"
//! through the run's `mcp-list-tools` / `mcp-call-tool` channels, which
//! the daemon relays to the app. What the app answers is here: the same
//! Space verbs a person clicks, and `ask_person` — a question, a choice
//! or a credential-by-meaning, put in front of the person as a card. The
//! agent waits for the answer; nothing times out. A credential's value
//! never reaches the agent: it learns that a key was used, not which.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use chrono::Utc;
use futures::stream;
use rmcp::model::{CallToolRequestParams, CallToolResult, ContentBlock, ErrorData, JsonObject, ListToolsResult, Tool};
use serde_json::{Value, json};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::daemon::Frames;
use crate::spaces::{Caller, Id, Spaces};
use crate::view::{CardEvent, CardKind, CardView};

struct Pending {
    view: CardView,
    answer: oneshot::Sender<String>,
}

pub struct Door {
    spaces: Arc<dyn Spaces>,
    cards: Mutex<Vec<Pending>>,
    live: broadcast::Sender<CardEvent>,
    next: AtomicU64,
    rt: tokio::runtime::Handle,
}

/// The stand-in vault: names only. The values are the daemon's, never the app's.
const VAULT: &[&str] = &["OpenRouter key", "Spotify key", "GitHub token", "Anthropic key"];

fn schema(props: Value, required: &[&str]) -> Arc<JsonObject> {
    let mut o = JsonObject::new();
    o.insert("type".into(), json!("object"));
    o.insert("properties".into(), props);
    o.insert("required".into(), json!(required));
    Arc::new(o)
}

impl Door {
    pub fn new(spaces: Arc<dyn Spaces>) -> Self {
        let (live, _) = broadcast::channel(64);
        Door { spaces, cards: Mutex::new(Vec::new()), live, next: AtomicU64::new(1), rt: tokio::runtime::Handle::current() }
    }

    pub fn tools(&self) -> ListToolsResult {
        ListToolsResult::with_all_items(vec![
            Tool::new("spaces_list", "The Spaces your person hosts or has joined.", schema(json!({}), &[])),
            Tool::new("space_feed", "What has happened in a Space: its moves, newest last.", schema(json!({ "space": { "type": "string" } }), &["space"])),
            Tool::new("space_tools", "A Space's own verbs, with their arguments.", schema(json!({ "space": { "type": "string" } }), &["space"])),
            Tool::new("space_call", "Do something in a Space: one of its verbs, with arguments. You act as yourself.", schema(json!({ "space": { "type": "string" }, "tool": { "type": "string" }, "arguments": { "type": "object" } }), &["space", "tool"])),
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

    async fn ask(&self, agent: &str, question: String, kind: CardKind, options: Vec<String>) -> String {
        let (tx, rx) = oneshot::channel();
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let view = CardView { id, agent: agent.to_owned(), kind, question, options, at: Utc::now().to_rfc3339() };
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
                let r = self.spaces.read(&id, crate::spaces::stub::rooms::FEED).await?;
                r.contents.iter().filter_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => Some(text.clone()), _ => None }).next().unwrap_or_default()
            }
            "space_tools" => {
                let id = Id { id: get("space").ok_or_else(|| ErrorData::invalid_params("space is needed", None))? };
                serde_json::to_string(&self.spaces.tools(&id).await?.tools).unwrap_or_default()
            }
            "space_call" => {
                let id = Id { id: get("space").ok_or_else(|| ErrorData::invalid_params("space is needed", None))? };
                let tool = get("tool").ok_or_else(|| ErrorData::invalid_params("tool is needed", None))?;
                let inner = CallToolRequestParams::new(tool).with_arguments(args.get("arguments").and_then(Value::as_object).cloned().unwrap_or_default());
                let result = self.spaces.call(&id, inner, Caller::Agent(agent.to_owned())).await?;
                result.content.iter().filter_map(|c| c.as_text().map(|t| t.text.clone())).collect::<Vec<_>>().join("\n")
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
