//! When one of your agents finishes a run, it reports to your Home — the
//! work is the social layer. The daemon's list is not live, so this asks
//! every few seconds and watches for active → inactive.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use rmcp::model::CallToolRequestParams;
use serde_json::json;

use diverge_sdk::daemon::endpoints::agents;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::{Item, ItemWrapper};
use diverge_sdk::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;

use crate::daemon::Daemon;
use crate::identity::{Actor, Identity};
use crate::spaces::Spaces;

/// The last turn's text and measurement, from the log after its last `active`.
pub async fn last_turn(daemon: &dyn Daemon, name: &str) -> (String, Option<String>) {
    let request = agents::logs::client::request::Frame { name: name.into(), logs_index_from: None, logs_index_to: None, created_from: None, created_to: None, r#type: None, jq: None, count: None, watch: None };
    let mut frames = daemon.agents_logs(request, tokio_util::sync::CancellationToken::new());
    let mut text = String::new();
    let mut measured = None;
    // Text arrives in pieces; a tool call between two stretches of text
    // means a new paragraph, so the report reads as the agent spoke.
    let mut paragraph_break = false;
    while let Some(frame) = frames.next().await {
        let agents::logs::server::response::Frame::Value(value) = frame else { continue };
        let Ok(w) = serde_json::from_value::<ItemWrapper>(value) else { continue };
        match w.item {
            Item::Active(_) => {
                text.clear();
                measured = None;
                paragraph_break = false;
            }
            Item::Chunk(AgenticLoopChunk::AssistantTextContent(c)) => {
                if let Some(t) = serde_json::to_value(&c.inner).ok().and_then(|v| v.get("text").and_then(|t| t.as_str()).map(str::to_owned)) {
                    if paragraph_break && !text.is_empty() {
                        text.push_str("\n\n");
                    }
                    paragraph_break = false;
                    text.push_str(&t);
                }
            }
            Item::Chunk(AgenticLoopChunk::Usage(u)) => measured = Some(format!("{} tokens", u.total_tokens)),
            Item::Chunk(_) => paragraph_break = true,
            Item::Error(_) | Item::Inactive(_) => {}
        }
    }
    (text, measured)
}

/// Whether Home has anyone in it besides you and your agents.
async fn home_is_yours_alone(spaces: &dyn Spaces, identity: &Identity, home: &crate::spaces::Id) -> bool {
    let Ok(r) = spaces.read(home, diverge_desktop_room::room::MEMBERS).await else { return false };
    let Some(text) = r.contents.iter().find_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => Some(text.clone()), _ => None }) else { return false };
    let members: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap_or_default();
    members.iter().all(|m| m["key"].as_str().is_some_and(|k| identity.owner_of(k).is_some()))
}

/// Where an agent's log ends now: the last item's index, or 0.
pub async fn log_end(daemon: &dyn Daemon, name: &str) -> u64 {
    let request = agents::logs::client::request::Frame { name: name.into(), logs_index_from: None, logs_index_to: None, created_from: None, created_to: None, r#type: None, jq: None, count: None, watch: None };
    let mut frames = daemon.agents_logs(request, tokio_util::sync::CancellationToken::new());
    let mut end = 0;
    while let Some(frame) = frames.next().await {
        let agents::logs::server::response::Frame::Value(value) = frame else { continue };
        if let Ok(w) = serde_json::from_value::<ItemWrapper>(value) {
            end = end.max(w.logs_index);
        }
    }
    end
}

/// The text of one message's run: the first message logged after `after`,
/// from its own parts up to the next message's, or the run's end. Nothing
/// from any other run, before or after, gets in.
pub async fn run_after(daemon: &dyn Daemon, name: &str, after: u64) -> (String, Option<String>) {
    let request = agents::logs::client::request::Frame { name: name.into(), logs_index_from: Some(after + 1), logs_index_to: None, created_from: None, created_to: None, r#type: None, jq: None, count: None, watch: None };
    let mut frames = daemon.agents_logs(request, tokio_util::sync::CancellationToken::new());
    let (mut text, mut measured, mut message, mut paragraph_break) = (String::new(), None, None::<String>, false);
    while let Some(frame) = frames.next().await {
        let agents::logs::server::response::Frame::Value(value) = frame else { continue };
        let Ok(w) = serde_json::from_value::<ItemWrapper>(value) else { continue };
        match w.item {
            Item::Chunk(chunk) => {
                let v = serde_json::to_value(&chunk).unwrap_or_default();
                let kind = v.get("type").and_then(|t| t.as_str()).unwrap_or_default();
                if kind.starts_with("user_") {
                    // Each message's parts carry its own key: another key is another message.
                    let key = v.get("key").and_then(|k| k.as_str()).map(str::to_owned);
                    match &message {
                        None => message = key.or(Some(String::new())),
                        Some(m) if key.as_deref().is_some_and(|k| k != m) => break,
                        _ => {}
                    }
                    continue;
                }
                if message.is_none() {
                    continue;
                }
                match chunk {
                    AgenticLoopChunk::AssistantTextContent(c) => {
                        if let Some(t) = serde_json::to_value(&c.inner).ok().and_then(|v| v.get("text").and_then(|t| t.as_str()).map(str::to_owned)) {
                            if paragraph_break && !text.is_empty() {
                                text.push_str("\n\n");
                            }
                            paragraph_break = false;
                            text.push_str(&t);
                        }
                    }
                    AgenticLoopChunk::Usage(u) => measured = Some(format!("{} tokens", u.total_tokens)),
                    _ => paragraph_break = true,
                }
            }
            Item::Inactive(_) if message.is_some() => break,
            _ => {}
        }
    }
    (text, measured)
}

pub fn spawn(daemon: Arc<dyn Daemon>, spaces: Arc<dyn Spaces>, identity: Arc<Identity>) {
    tauri::async_runtime::spawn(async move {
        let mut seen: HashMap<String, bool> = HashMap::new();
        loop {
            tokio::time::sleep(Duration::from_millis(2500)).await;
            let listed = daemon.agents_list(agents::list::client::request::Frame {}).collect::<Vec<_>>().await;
            for frame in listed {
                let agents::list::server::response::Frame::Agent(a) = frame else { continue };
                let was = seen.insert(a.name.clone(), a.active);
                if was == Some(true) && !a.active {
                    let Some(home) = spaces.home().await else { continue };
                    let (text, measured) = last_turn(daemon.as_ref(), &a.name).await;
                    // A run's words go only where nobody else reads them: with others in Home, just that it finished.
                    let body: String = if home_is_yours_alone(spaces.as_ref(), &identity, &home).await { text.trim().chars().take(280).collect() } else { String::new() };
                    let mut args = json!({ "title": format!("{} finished a run", a.name), "body": body });
                    if let Some(m) = measured {
                        args["measured"] = json!(m);
                    }
                    let mut params = CallToolRequestParams::new("report").with_arguments(args.as_object().cloned().unwrap_or_default());
                    let actor = Actor::Agent(crate::identity::AgentId::Daemon(a.name.clone()));
                    let turn = identity.turn(&actor, &home.id);
                    let _held = turn.lock().await;
                    if identity.seal(&actor, &home.id, &mut params).is_ok() {
                        let _ = spaces.call(&home, params).await;
                    }
                }
            }
        }
    });
}
