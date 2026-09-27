//! The browser preview's snapshot: what the stand-in daemon says, written
//! to `src/preview/fixture.json` so the screens can be looked at in any
//! browser (reviews, cloud sessions) without the Mac window. Nothing in
//! the preview runs; it plays this snapshot back.
//!
//! Regenerate: `cargo test -p diverge-desktop export_preview_fixture`.

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use futures::StreamExt;
    use serde_json::json;
    use tokio_util::sync::CancellationToken;

    use diverge_sdk::daemon::endpoints::agents;
    use diverge_sdk::provider::endpoints::volumes as pv;

    use crate::daemon::Daemon;
    use crate::daemon::stub::StubDaemon;
    use crate::machines::{Machines, ReadFrame};
    use crate::spaces::stub::{rooms, StubSpaces};
    use crate::spaces::Spaces;
    use crate::view::*;

    fn files(nodes: &[FileNode], prefix: &str, out: &mut Vec<String>) {
        for node in nodes {
            match node {
                FileNode::File { name, .. } => out.push(format!("{prefix}/{name}")),
                FileNode::Directory { name, children, .. } => files(children, &format!("{prefix}/{name}"), out),
                FileNode::Symlink { .. } => {}
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn export_preview_fixture() {
        let root = std::env::temp_dir().join(format!("diverge-desktop-preview-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let daemon = StubDaemon::new(root);
        let spaces = StubSpaces::new();
        // Let the long job get part of the way through (virtual time).
        tokio::time::sleep(Duration::from_secs(70)).await;

        let listed = listed(daemon.agents_list(agents::list::client::request::Frame {}).collect().await);
        let mut logs = BTreeMap::new();
        for agent in &listed.agents {
            let request = agents::logs::client::request::Frame {
                name: agent.name.clone(),
                logs_index_from: None,
                logs_index_to: None,
                created_from: None,
                created_to: None,
                r#type: None,
                jq: None,
                count: None,
                watch: None,
            };
            let entries: Vec<LogEntry> = daemon
                .agents_logs(request, CancellationToken::new())
                .map(LogEvent::from)
                .filter_map(|e| async move { if let LogEvent::Entry { entry } = e { Some(entry) } else { None } })
                .collect()
                .await;
            logs.insert(agent.name.clone(), entries);
        }

        // Each machine's volumes, trees and files, keyed by the machine's
        // identity key, then "volume:path" for a file.
        let mut machines: Vec<MachineView> = Vec::new();
        let mut trees = BTreeMap::new();
        let mut texts = BTreeMap::new();
        let mut stats = BTreeMap::new();
        for p in daemon.providers_list().await {
            let on = p.identity.clone();
            let identity: ProviderView = (&p.identity).into();
            let key = identity_key(&identity);
            let (volumes, volumes_problem) = match VolumesListed::from(daemon.volumes_list(&on).await) {
                VolumesListed::Volumes { volumes } => (volumes, None),
                VolumesListed::Error { message } => (Vec::new(), Some(message)),
            };
            let mut machine_trees = BTreeMap::new();
            let mut machine_texts = BTreeMap::new();
            let mut machine_stats = BTreeMap::new();
            for v in &volumes {
                let stat: VolumeStat = daemon.volumes_stat(&on, pv::stat::client::request::Frame { name: v.name.clone() }).await.into();
                machine_stats.insert(v.name.clone(), stat);
                // A held volume has no tree to show: the preview says why, as the app does.
                let tree: VolumeTree = daemon.volumes_filetree(&on, pv::filetree::client::request::Frame { name: v.name.clone(), path: Vec::new() }).await.into();
                let VolumeTree::Tree { nodes } = tree else { continue };
                let mut paths = Vec::new();
                files(&nodes, "", &mut paths);
                for path in paths {
                    let comps: Vec<String> = path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect();
                    let mut bytes = Vec::new();
                    let mut frames = daemon.volumes_read(&on, pv::read::client::request::Frame { name: v.name.clone(), path: comps });
                    while let Some(ReadFrame::Body(body)) = frames.next().await {
                        bytes.extend(body);
                    }
                    machine_texts.insert(format!("{}:{}", v.name, path), String::from_utf8_lossy(&bytes).into_owned());
                }
                machine_trees.insert(v.name.clone(), nodes);
            }
            trees.insert(key.clone(), machine_trees);
            stats.insert(key.clone(), machine_stats);
            texts.insert(key, machine_texts);
            machines.push(MachineView { identity, volumes, volumes_problem, added: p.added.to_rfc3339(), name: None });
        }

        let door = std::sync::Arc::new(crate::door::Door::new(std::sync::Arc::new(StubSpaces::new())));
        let door_tools: Vec<ToolView> = door.tools().tools.iter().map(Into::into).collect();
        {
            let door = door.clone();
            tokio::spawn(async move {
                let params = rmcp::model::CallToolRequestParams::new("ask_person").with_arguments(json!({ "question": "Claim “Package the photo resizer as a tool” on the Saturday Workshop board? Claiming is a commitment to its spec.", "kind": "choice", "options": ["Yes, claim it", "No, leave it"] }).as_object().cloned().unwrap());
                let _ = door.call("site-fixes", params).await;
            });
        }
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;
        let cards = door.cards();

        let mut space_views = Vec::new();
        for e in spaces.list().await {
            let text = |r: rmcp::model::ReadResourceResult| r.contents.iter().filter_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => Some(text.clone()), _ => None }).next().unwrap_or_default();
            let tools = spaces.tools(&e.id).await.unwrap();
            let members: Vec<MemberView> = serde_json::from_str(&text(spaces.read(&e.id, rooms::MEMBERS).await.unwrap())).unwrap();
            let moves: Vec<MoveView> = serde_json::from_str(&text(spaces.read(&e.id, rooms::FEED).await.unwrap())).unwrap();
            let charter = text(spaces.read(&e.id, rooms::CHARTER).await.unwrap());
            let view = SpaceView { summary: (&e).into(), charter, members, tools: tools.tools.iter().map(Into::into).collect() };
            let invite = spaces.invite(&e.id).await.map(|i| crate::actions::invite_text(&i));
            space_views.push(json!({ "view": view, "moves": moves, "invite": invite }));
        }
        let cancel = CancellationToken::new();
        let mut knock_stream = spaces.knocks(cancel.clone());
        let mut knocks = Vec::new();
        while let Ok(Some(k)) = tokio::time::timeout(Duration::from_millis(50), knock_stream.next()).await {
            let title = spaces.list().await.into_iter().find(|e| e.id == k.space).map(|e| e.title).unwrap_or_default();
            knocks.push(KnockView { knock_id: k.knock_id, space: k.space.id.clone(), space_title: title, address: k.authorize.address.to_string(), authorization: k.authorize.authorization.clone(), at: k.at.to_rfc3339() });
        }
        cancel.cancel();

        let fixture = json!({
            "note": "Generated by `cargo test -p diverge-desktop export_preview_fixture`. Do not edit.",
            "contract_pin": include_str!("../../CONTRACT_PIN").lines().next().unwrap_or_default(),
            "actions": crate::actions::actions_list(),
            "catalog": crate::actions::catalog_images(),
            "agents": listed.agents,
            "logs": logs,
            "machines": machines,
            "spaces": space_views,
            "door_tools": door_tools,
            "cards": cards,
            "knocks": knocks,
            "trees": trees,
            "stats": stats,
            "mounts": daemon.creates().iter().map(|c| (c.name.clone(), AgentMounts::of_create(c).view())).collect::<BTreeMap<_, _>>(),
            "files": texts,
        });
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/preview/fixture.json");
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        std::fs::write(out, serde_json::to_string_pretty(&fixture).unwrap()).unwrap();
    }
}
