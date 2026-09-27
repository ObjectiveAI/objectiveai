//! A stand-in room: what a Space's tool container would answer, kept in
//! memory. Its verbs are MCP tools with JSON Schemas (the screen renders
//! them with the same form system the agent images use), its objects are
//! *moves* served as the `space://feed` resource, and every change is an
//! MCP resource-updated notification.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ContentBlock, ErrorData, JsonObject, ListToolsResult, Notification, ReadResourceResult,
    ResourceContents, ResourceUpdatedNotificationParam, ServerNotification, Tool,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tokio::sync::broadcast;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;

pub const FEED: &str = "space://feed";
pub const CHARTER: &str = "space://charter";
pub const MEMBERS: &str = "space://members";

/// The name this daemon's person goes by in every room here.
pub const ME: &str = "me";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Home,
    Board,
    Idea,
    Dm,
}

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Home => "home",
            Kind::Board => "board",
            Kind::Idea => "idea",
            Kind::Dm => "dm",
        }
    }
    pub fn parse(s: &str) -> Option<Kind> {
        [Kind::Home, Kind::Board, Kind::Idea, Kind::Dm].into_iter().find(|k| k.key() == s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub name: String,
    pub is_agent: bool,
    pub joined: DateTime<Utc>,
}

/// One object in a room: a show, an ask, a task, a reply, a run…
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Move {
    pub id: String,
    pub kind: String,
    pub author: String,
    pub at: DateTime<Utc>,
    pub title: String,
    pub body: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default)]
    pub fields: Map<String, Value>,
}

pub struct Room {
    pub id: String,
    pub title: String,
    pub kind: Kind,
    pub host: Identity,
    pub mine: bool,
    pub online: bool,
    pub charter: String,
    /// What a joiner must present (before " as <name>").
    pub invite: String,
    pub joined_as: String,
    pub members: Vec<Member>,
    pub moves: Vec<Move>,
    pub live: broadcast::Sender<ServerNotification>,
    next: u64,
}

fn schema(props: Value, required: &[&str]) -> Arc<JsonObject> {
    let mut o = JsonObject::new();
    o.insert("type".into(), json!("object"));
    o.insert("properties".into(), props);
    o.insert("required".into(), json!(required));
    Arc::new(o)
}

impl Room {
    pub fn new(id: &str, title: &str, kind: Kind, host: Identity, mine: bool, charter: &str, invite: &str, joined_as: &str) -> Self {
        let (live, _) = broadcast::channel(256);
        Room {
            id: id.into(),
            title: title.into(),
            kind,
            host,
            mine,
            online: true,
            charter: charter.into(),
            invite: invite.into(),
            joined_as: joined_as.into(),
            members: Vec::new(),
            moves: Vec::new(),
            live,
            next: 1,
        }
    }

    pub fn member(&mut self, name: &str, is_agent: bool, joined: DateTime<Utc>) -> &mut Self {
        if !self.members.iter().any(|m| m.name == name) {
            self.members.push(Member { name: name.into(), is_agent, joined });
        }
        self
    }

    pub fn push(&mut self, at: DateTime<Utc>, author: &str, kind: &str, title: &str, body: &str, state: &str, parent: Option<&str>, fields: Map<String, Value>) -> String {
        let id = format!("{}-{}", kind, self.next);
        self.next += 1;
        self.moves.push(Move { id: id.clone(), kind: kind.into(), author: author.into(), at, title: title.into(), body: body.into(), state: state.into(), parent: parent.map(str::to_owned), fields });
        id
    }

    fn notify(&self, uri: &str) {
        let _ = self.live.send(ServerNotification::ResourceUpdatedNotification(Notification::new(ResourceUpdatedNotificationParam::new(uri))));
    }

    /// The room's verbs, as MCP tools: what a person clicks and an agent calls.
    pub fn tools(&self) -> ListToolsResult {
        let mut tools = vec![
            Tool::new("show", "Show something you made or did.", schema(json!({ "title": { "type": "string" }, "body": { "type": "string", "description": "What it is, in your words." } }), &["title"])),
            Tool::new(
                "ask",
                "Ask for something. Anyone here — a person or an agent — may serve it.",
                schema(
                    json!({
                        "what": { "type": "string", "description": "What you need." },
                        "needs": { "type": "string", "description": "What serving it takes: skills, access, hardware." },
                        "ceiling": { "type": "string", "description": "The most you'd spend on it — time, tokens, or a budget." },
                        "who_may_serve": { "type": "string", "enum": ["anyone", "members", "agents"], "description": "Who may pick it up." }
                    }),
                    &["what"],
                ),
            ),
            Tool::new("reply", "Reply to a move.", schema(json!({ "move_id": { "type": "string" }, "body": { "type": "string" } }), &["move_id", "body"])),
        ];
        if self.kind == Kind::Home {
            tools.push(Tool::new("report", "Report what you finished — for an agent, when a run ends.", schema(json!({ "title": { "type": "string" }, "body": { "type": "string" }, "measured": { "type": "string", "description": "What the run measured: tokens, seconds." } }), &["title"])));
        }
        if self.mine && matches!(self.kind, Kind::Home | Kind::Board) {
            tools.push(Tool::new("admit", "Host only: add one of your own agents (or a person you vouch for) as a member.", schema(json!({ "name": { "type": "string" }, "agent": { "type": "boolean" } }), &["name"])));
        }
        match self.kind {
            Kind::Board => tools.extend([
                Tool::new(
                    "post_task",
                    "Post a task: its spec is fixed at posting, and whoever claims it is judged on the spec.",
                    schema(
                        json!({
                            "title": { "type": "string" },
                            "spec": { "type": "string", "description": "What done looks like. The spec is what gets checked, not the hours." }
                        }),
                        &["title", "spec"],
                    ),
                ),
                Tool::new("claim", "Claim an open task. You take it as posted.", schema(json!({ "task_id": { "type": "string" } }), &["task_id"])),
                Tool::new("deliver", "Deliver on a task you claimed.", schema(json!({ "task_id": { "type": "string" }, "summary": { "type": "string", "description": "What you did, against the spec." } }), &["task_id", "summary"])),
                Tool::new("accept", "Accept a delivery: the task is done and a receipt is issued.", schema(json!({ "task_id": { "type": "string" } }), &["task_id"])),
                Tool::new(
                    "post_offering",
                    "Offer something you sell or lend: a fixed thing, or a metered capability.",
                    schema(
                        json!({
                            "title": { "type": "string" },
                            "what": { "type": "string" },
                            "pricing": { "type": "string", "enum": ["fixed", "metered"], "description": "Fixed: one price, instant. Metered: priced in what the wire counts." },
                            "terms": { "type": "string" }
                        }),
                        &["title", "what", "pricing"],
                    ),
                ),
            ]),
            Kind::Idea => tools.extend([
                Tool::new("propose", "Propose a direction the idea could go.", schema(json!({ "direction": { "type": "string" }, "body": { "type": "string" } }), &["direction"])),
                Tool::new(
                    "steer",
                    "Steer a direction: prefer it, reject it, or leave a note. Steering compiles into the next round.",
                    schema(json!({ "direction_id": { "type": "string" }, "move": { "type": "string", "enum": ["prefer", "reject", "note"] }, "note": { "type": "string" } }), &["direction_id", "move"]),
                ),
                Tool::new("synthesize", "Write where the idea has landed so far. It never closes; the next round starts from here.", schema(json!({ "body": { "type": "string" } }), &["body"])),
            ]),
            Kind::Dm => {
                tools.retain(|t| t.name == "reply");
                tools.insert(0, Tool::new("say", "Say something.", schema(json!({ "body": { "type": "string" } }), &["body"])));
            }
            Kind::Home => {}
        }
        ListToolsResult::with_all_items(tools)
    }

    pub fn read(&self, uri: &str) -> Result<ReadResourceResult, ErrorData> {
        let (text, mime) = match uri {
            FEED => (serde_json::to_string(&self.moves).unwrap_or_default(), "application/json"),
            MEMBERS => (serde_json::to_string(&self.members).unwrap_or_default(), "application/json"),
            CHARTER => (self.charter.clone(), "text/markdown"),
            _ => return Err(ErrorData::resource_not_found(format!("no resource at {uri}"), None)),
        };
        Ok(ReadResourceResult::new(vec![ResourceContents::TextResourceContents { uri: uri.into(), mime_type: Some(mime.into()), text, meta: None }]))
    }

    fn arg<'a>(args: &'a JsonObject, key: &str) -> Option<&'a str> {
        args.get(key).and_then(Value::as_str).filter(|s| !s.trim().is_empty())
    }

    fn need<'a>(args: &'a JsonObject, key: &str) -> Result<&'a str, ErrorData> {
        Self::arg(args, key).ok_or_else(|| ErrorData::invalid_params(format!("{key} is needed"), None))
    }

    /// A tool called: the room changes, everyone watching hears, and the
    /// caller gets one line back.
    pub fn call(&mut self, params: CallToolRequestParams, author: &str) -> Result<CallToolResult, ErrorData> {
        let now = Utc::now();
        let args = params.arguments.unwrap_or_default();
        let name = params.name.to_string();
        let allowed = self.tools().tools.iter().any(|t| t.name == name);
        if !allowed {
            return Err(ErrorData::invalid_params(format!("this room has no verb called {name}"), None));
        }
        if !self.members.iter().any(|m| m.name == author) {
            return Err(ErrorData::invalid_request(format!("{author} is not a member here"), None));
        }
        let line = match name.as_str() {
            "report" => {
                let title = Self::need(&args, "title")?;
                let mut fields = Map::new();
                if let Some(m) = Self::arg(&args, "measured") {
                    fields.insert("measured".into(), json!(m));
                }
                let id = self.push(now, author, "run", title, Self::arg(&args, "body").unwrap_or_default(), "done", None, fields);
                format!("Reported: {title} ({id})")
            }
            "admit" => {
                if author != ME {
                    return Err(ErrorData::invalid_request("only the host may admit", None));
                }
                let who = Self::need(&args, "name")?.to_owned();
                let agent = args.get("agent").and_then(Value::as_bool).unwrap_or(false);
                self.join(&who, agent);
                format!("Admitted: {who}")
            }
            "show" => {
                let title = Self::need(&args, "title")?;
                let id = self.push(now, author, "show", title, Self::arg(&args, "body").unwrap_or_default(), "shown", None, Map::new());
                format!("Shown: {title} ({id})")
            }
            "ask" => {
                let what = Self::need(&args, "what")?;
                let mut fields = Map::new();
                for k in ["needs", "ceiling", "who_may_serve"] {
                    if let Some(v) = Self::arg(&args, k) {
                        fields.insert(k.into(), json!(v));
                    }
                }
                let id = self.push(now, author, "ask", what, "", "open", None, fields);
                format!("Asked: {what} ({id})")
            }
            "say" => {
                let body = Self::need(&args, "body")?;
                let id = self.push(now, author, "say", "", body, "said", None, Map::new());
                format!("Said ({id})")
            }
            "reply" => {
                let parent = Self::need(&args, "move_id")?.to_owned();
                if !self.moves.iter().any(|m| m.id == parent) {
                    return Err(ErrorData::invalid_params(format!("no move called {parent}"), None));
                }
                let body = Self::need(&args, "body")?;
                let id = self.push(now, author, "reply", "", body, "said", Some(&parent), Map::new());
                format!("Replied to {parent} ({id})")
            }
            "post_task" => {
                let title = Self::need(&args, "title")?;
                let spec = Self::need(&args, "spec")?;
                let id = self.push(now, author, "task", title, spec, "open", None, Map::new());
                format!("Task posted: {title} ({id})")
            }
            "claim" => {
                let task = Self::need(&args, "task_id")?.to_owned();
                let m = self.moves.iter_mut().find(|m| m.id == task && m.kind == "task").ok_or_else(|| ErrorData::invalid_params(format!("no task called {task}"), None))?;
                if m.state != "open" {
                    return Err(ErrorData::invalid_request(format!("{task} is {}", m.state), None));
                }
                m.state = "claimed".into();
                m.fields.insert("claimed_by".into(), json!(author));
                let title = m.title.clone();
                self.push(now, author, "claim", &title, "", "claimed", Some(&task), Map::new());
                format!("Claimed: {title}")
            }
            "deliver" => {
                let task = Self::need(&args, "task_id")?.to_owned();
                let summary = Self::need(&args, "summary")?.to_owned();
                let m = self.moves.iter_mut().find(|m| m.id == task && m.kind == "task").ok_or_else(|| ErrorData::invalid_params(format!("no task called {task}"), None))?;
                if m.fields.get("claimed_by").and_then(Value::as_str) != Some(author) {
                    return Err(ErrorData::invalid_request(format!("{author} did not claim {task}"), None));
                }
                m.state = "delivered".into();
                let title = m.title.clone();
                self.push(now, author, "delivery", &title, &summary, "delivered", Some(&task), Map::new());
                format!("Delivered: {title}")
            }
            "accept" => {
                let task = Self::need(&args, "task_id")?.to_owned();
                let m = self.moves.iter_mut().find(|m| m.id == task && m.kind == "task").ok_or_else(|| ErrorData::invalid_params(format!("no task called {task}"), None))?;
                if m.author != author {
                    return Err(ErrorData::invalid_request("only who posted a task may accept its delivery", None));
                }
                if m.state != "delivered" {
                    return Err(ErrorData::invalid_request(format!("{task} is {}", m.state), None));
                }
                m.state = "done".into();
                let (title, who) = (m.title.clone(), m.fields.get("claimed_by").and_then(Value::as_str).unwrap_or_default().to_owned());
                let mut fields = Map::new();
                fields.insert("to".into(), json!(who));
                self.push(now, author, "receipt", &title, &format!("Completed: {title}"), "issued", Some(&task), fields);
                format!("Accepted: {title}")
            }
            "post_offering" => {
                let title = Self::need(&args, "title")?;
                let mut fields = Map::new();
                fields.insert("pricing".into(), json!(Self::need(&args, "pricing")?));
                if let Some(v) = Self::arg(&args, "terms") {
                    fields.insert("terms".into(), json!(v));
                }
                let id = self.push(now, author, "offering", title, Self::need(&args, "what")?, "offered", None, fields);
                format!("Offered: {title} ({id})")
            }
            "propose" => {
                let direction = Self::need(&args, "direction")?;
                let id = self.push(now, author, "direction", direction, Self::arg(&args, "body").unwrap_or_default(), "open", None, Map::new());
                format!("Proposed: {direction} ({id})")
            }
            "steer" => {
                let target = Self::need(&args, "direction_id")?.to_owned();
                let mv = Self::need(&args, "move")?.to_owned();
                let m = self.moves.iter_mut().find(|m| m.id == target && m.kind == "direction").ok_or_else(|| ErrorData::invalid_params(format!("no direction called {target}"), None))?;
                let n = m.fields.get(&mv).and_then(Value::as_u64).unwrap_or(0) + 1;
                m.fields.insert(mv.clone(), json!(n));
                let title = m.title.clone();
                self.push(now, author, "steer", &title, Self::arg(&args, "note").unwrap_or_default(), &mv, Some(&target), Map::new());
                format!("Steered {title}: {mv}")
            }
            "synthesize" => {
                let body = Self::need(&args, "body")?;
                let id = self.push(now, author, "synthesis", "Where it stands", body, "open", None, Map::new());
                format!("Synthesis written ({id})")
            }
            other => return Err(ErrorData::invalid_params(format!("this room has no verb called {other}"), None)),
        };
        self.notify(FEED);
        Ok(CallToolResult::success(vec![ContentBlock::text(line)]))
    }

    pub fn join(&mut self, name: &str, is_agent: bool) {
        self.member(name, is_agent, Utc::now());
        self.notify(MEMBERS);
    }
}
