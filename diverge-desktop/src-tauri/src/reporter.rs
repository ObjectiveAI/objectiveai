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
use crate::spaces::{Caller, Spaces};

/// The last turn's text and measurement, from the log after its last `active`.
async fn last_turn(daemon: &dyn Daemon, name: &str) -> (String, Option<String>) {
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

pub fn spawn(daemon: Arc<dyn Daemon>, spaces: Arc<dyn Spaces>) {
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
                    let body: String = text.trim().chars().take(280).collect();
                    let mut args = json!({ "title": format!("{} finished a run", a.name), "body": body });
                    if let Some(m) = measured {
                        args["measured"] = json!(m);
                    }
                    let params = CallToolRequestParams::new("report").with_arguments(args.as_object().cloned().unwrap_or_default());
                    let _ = spaces.call(&home, params, Caller::Agent(a.name.clone())).await;
                }
            }
        }
    });
}
