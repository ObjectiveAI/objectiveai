//! Hires: someone asks one of your agents for something, through your
//! profile room. The room calls its host (on the wire, the container's own
//! `mcp-call-tool` to its runner: this app). You decide on a card; your
//! agent does the work on your machine; the result goes on the profile's
//! table and is delivered. Nobody else reaches your agent's container,
//! which the wire forbids anyway: "an agent container is its runner's alone".

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use diverge_sdk::daemon::endpoints::agents;

use crate::daemon::Daemon;
use crate::door::Door;
use crate::identity::Identity;
use crate::spaces::{HostCall, Id, Spaces};
use crate::view::CardKind;

pub const TAKE: &str = "Take it";
pub const DECLINE: &str = "Turn it down";

pub fn spawn(daemon: Arc<dyn Daemon>, spaces: Arc<dyn Spaces>, identity: Arc<Identity>, door: Arc<Door>) {
    tauri::async_runtime::spawn(async move {
        let mut calls = spaces.host_calls(CancellationToken::new());
        while let Some(call) = calls.next().await {
            let (daemon, spaces, identity, door) = (daemon.clone(), spaces.clone(), identity.clone(), door.clone());
            tauri::async_runtime::spawn(async move { handle(daemon, spaces, identity, door, call).await });
        }
    });
}

async fn seal_call(spaces: &dyn Spaces, identity: &Identity, room: &Id, verb: &str, args: Value) -> Result<(), String> {
    let mut params = CallToolRequestParams::new(verb.to_owned()).with_arguments(args.as_object().cloned().unwrap_or_default());
    identity.seal(&identity.you_in(&room.id), &room.id, &mut params)?;
    spaces.call(room, params).await.map(|_| ()).map_err(|e| e.message.to_string())
}

pub async fn handle(daemon: Arc<dyn Daemon>, spaces: Arc<dyn Spaces>, identity: Arc<Identity>, door: Arc<Door>, call: HostCall) {
    let HostCall::Hire { room, hire_id, from, agent, what, pledge } = call;
    let pledge = pledge.map(|p| format!(" They pledge: {p}.")).unwrap_or_default();
    let question = format!("{from} asks {agent}, through your profile: “{what}”.{pledge} It would run on your machine.");
    let answer = door.ask_for(&agent, &from, question, CardKind::Choice, vec![TAKE.into(), DECLINE.into()]).await;
    let take = answer == TAKE;
    if seal_call(spaces.as_ref(), &identity, &room, "answer_hire", json!({ "hire_id": hire_id, "take": take })).await.is_err() || !take {
        return;
    }
    // The agent does the work, as for any message of yours.
    let content = vec![serde_json::from_value(json!({ "type": "text", "text": format!("For {from}, who asked through my profile: {what}") })).expect("a text block")];
    let sent = daemon.agents_message(agents::message::client::request::Frame { name: agent.clone(), content }, CancellationToken::new()).await;
    if !matches!(sent, agents::message::server::response::Frame::Delivered) {
        return;
    }
    // Wait for the run to end: the daemon's list is not live.
    let mut seen_active = false;
    for _ in 0..720 {
        tokio::time::sleep(Duration::from_millis(2500)).await;
        let listed = crate::view::listed(daemon.agents_list(agents::list::client::request::Frame {}).collect::<Vec<_>>().await);
        let Some(a) = listed.agents.into_iter().find(|a| a.name == agent) else { return };
        if a.active {
            seen_active = true;
        } else if seen_active {
            break;
        }
    }
    let (text, _) = crate::reporter::last_turn(daemon.as_ref(), &agent).await;
    let path = vec![format!("for-{}", from.replace(['/', ' '], "-")), format!("{hire_id}.md")];
    let body = format!("# {what}\n\nFor {from}, by {agent}.\n\n{text}\n");
    if spaces.table_write(&room, &path, body.into_bytes()).await.is_err() {
        return;
    }
    let summary = text.split(['\n', '.']).find(|l| !l.trim().is_empty()).unwrap_or("Done").trim().to_owned();
    let _ = seal_call(spaces.as_ref(), &identity, &room, "deliver_hire", json!({ "hire_id": hire_id, "summary": summary, "files": [path.join("/")] })).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::stub::StubDaemon;
    use crate::spaces::stub::StubSpaces;

    /// ren hires an agent through the profile; you take it; the agent runs;
    /// the result lands on the profile's table and the hire is delivered.
    #[tokio::test(start_paused = true)]
    async fn a_hire_taken_runs_and_is_delivered() {
        let root = std::env::temp_dir().join(format!("diverge-desktop-test-hire-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let identity = Arc::new(Identity::stand_in("maya"));
        let daemon = Arc::new(StubDaemon::new(root.join("host")));
        let stub = StubSpaces::new(identity.clone(), root.join("tables"));
        let spaces: Arc<dyn Spaces> = Arc::new(stub.clone());
        let door = Arc::new(Door::new(spaces.clone(), identity.clone(), None));
        daemon.set_door(door.clone());
        let mut calls = spaces.host_calls(CancellationToken::new());
        let profile = Id { id: stub.id_of("profile-me") };
        stub.act_now("ren", &profile.id, "hire", json!({ "agent": "research-notes", "what": "Check the links on my music page", "pledge": "a coffee" })).unwrap();
        let call = calls.next().await.unwrap();
        let hire = {
            let HostCall::Hire { hire_id, .. } = &call;
            hire_id.clone()
        };
        let running = tokio::spawn(handle(daemon.clone(), spaces.clone(), identity.clone(), door.clone(), call));
        let card = loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if let Some(c) = door.cards().into_iter().next() {
                break c;
            }
        };
        assert!(card.question.contains("ren asks research-notes"), "{}", card.question);
        door.answer(card.id, TAKE.into()).unwrap();
        running.await.unwrap();
        let path = vec!["for-ren".to_owned(), format!("{hire}.md")];
        let body = String::from_utf8(spaces.table_read(&profile, &path).await.unwrap()).unwrap();
        assert!(body.contains("one broken"), "the agent's answer is on the table: {body}");
        let feed = spaces.read(&profile, diverge_desktop_room::room::FEED).await.unwrap();
        let rmcp::model::ResourceContents::TextResourceContents { text, .. } = &feed.contents[0] else { panic!() };
        let moves: Vec<Value> = serde_json::from_str(text).unwrap();
        assert_eq!(moves.iter().find(|m| m["id"] == hire.as_str()).unwrap()["state"], "delivered");
    }
}
