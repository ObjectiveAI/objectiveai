//! The browser preview's snapshot: what the stand-in daemon says, written
//! to `src/preview/fixture.json` so the screens can be looked at in any
//! browser (reviews, cloud sessions) without the Mac window. Nothing in
//! the preview runs; it plays this snapshot back.
//!
//! Regenerate: `DIVERGE_EXPORT_PREVIEW=1 cargo test -p diverge-desktop export_preview_fixture`.
//! Without it the test still builds the snapshot, so it can't quietly break,
//! but leaves the committed file alone: seals over times differ every run.

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
    use crate::spaces::stub::StubSpaces;
    use crate::spaces::Spaces;
    use diverge_desktop_room::room as program;
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
        let daemon = StubDaemon::new(root.clone());
        let identity = std::sync::Arc::new(crate::identity::Identity::stand_in("maya"));
        let _ = std::fs::remove_dir_all(root.join("tables"));
        let spaces = StubSpaces::new(identity.clone(), root.join("tables"));
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

        let door = std::sync::Arc::new(crate::door::Door::new(std::sync::Arc::new(spaces.clone()), identity.clone(), None));
        let door_tools: Vec<ToolView> = door.tools().tools.iter().map(Into::into).collect();
        {
            let door = door.clone();
            tokio::spawn(async move {
                let params = rmcp::model::CallToolRequestParams::new("space_call").with_arguments(
                    json!({ "space": crate::spaces::stub::board(), "tool": "claim", "arguments": { "task_id": crate::spaces::stub::open_task() } }).as_object().cloned().unwrap(),
                );
                let _ = door.call("site-fixes", params).await;
            });
        }
        // ren hires research-notes through your profile: the hire, and its card, as the app raises them.
        {
            let profile = spaces.id_of("profile-me");
            spaces.act_now("ren", &profile, "hire", json!({ "agent": "research-notes", "what": "Check the links on my music page", "pledge": "a copy of the next track" })).unwrap();
            let door = door.clone();
            tokio::spawn(async move {
                let hire = crate::view::CardHire { from: "ren".into(), what: "Check the links on my music page".into(), pledge: Some("a copy of the next track".into()) };
                door.ask_hire("research-notes", hire, vec![crate::hires::TAKE.into(), crate::hires::DECLINE.into()]).await
            });
        }
        tokio::task::yield_now().await;
        for _ in 0..20 {
            tokio::task::yield_now().await;
            if door.cards().len() >= 2 {
                break;
            }
        }
        let cards = door.cards();

        let mut space_views = Vec::new();
        let mut tables = BTreeMap::new();
        let copies: BTreeMap<String, diverge_desktop_room::Record> = spaces.records_you_hold().into_iter().collect();
        for e in spaces.list().await {
            if !e.online {
                // Unreachable: the preview shows your copy, as the app does.
                let copy = copies.get(&e.id.id).cloned().unwrap();
                let room = diverge_desktop_room::Room::check(&copy).unwrap();
                let rmcp::model::ResourceContents::TextResourceContents { text, .. } = &room.read(program::FEED).unwrap().contents[0] else { panic!() };
                let moves: Vec<MoveView> = serde_json::from_str(text).unwrap();
                let rmcp::model::ResourceContents::TextResourceContents { text: who, .. } = &room.read(program::MEMBERS).unwrap().contents[0] else { panic!() };
                let members: Vec<MemberView> = serde_json::from_str(who).unwrap();
                let view = SpaceView { summary: summary(&e, &identity), charter: room.charter().to_owned(), members, tools: Vec::new(), before: Vec::new() };
                space_views.push(json!({ "view": view, "moves": moves, "invite": null, "from_copy": true }));
                continue;
            }
            let text = |r: rmcp::model::ReadResourceResult| r.contents.iter().filter_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => Some(text.clone()), _ => None }).next().unwrap_or_default();
            let tools = spaces.tools(&e.id).await.unwrap();
            let members: Vec<MemberView> = serde_json::from_str(&text(spaces.read(&e.id, program::MEMBERS).await.unwrap())).unwrap();
            let moves: Vec<MoveView> = serde_json::from_str(&text(spaces.read(&e.id, program::FEED).await.unwrap())).unwrap();
            let charter = text(spaces.read(&e.id, program::CHARTER).await.unwrap());
            let view = SpaceView { summary: summary(&e, &identity), charter, members, tools: tools.tools.iter().map(Into::into).collect(), before: Vec::new() };
            let invite = spaces.invite(&e.id).await.map(|i| i.to_text());
            space_views.push(json!({ "view": view, "moves": moves, "invite": invite }));
            // The table: its tree, and every text file on it.
            let nodes: Vec<FileNode> = spaces.table_tree(&e.id).await.map(|n| n.iter().map(Into::into).collect()).unwrap_or_default();
            let mut paths = Vec::new();
            files(&nodes, "", &mut paths);
            let mut texts = BTreeMap::new();
            for path in paths {
                let parts: Vec<String> = path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect();
                if let Ok(bytes) = spaces.table_read(&e.id, &parts).await {
                    texts.insert(path.trim_start_matches('/').to_owned(), String::from_utf8_lossy(&bytes).into_owned());
                }
            }
            tables.insert(e.id.id.clone(), json!({ "nodes": nodes, "files": texts }));
        }
        let cancel = CancellationToken::new();
        let mut knock_stream = spaces.knocks(cancel.clone());
        let mut knocks = Vec::new();
        while let Ok(Some(k)) = tokio::time::timeout(Duration::from_millis(50), knock_stream.next()).await {
            let title = spaces.list().await.into_iter().find(|e| e.id == k.space).map(|e| e.title).unwrap_or_default();
            let secret = spaces.invite(&k.space).await.and_then(|i| i.secret);
            let members: Vec<MemberView> = spaces
                .read(&k.space, program::MEMBERS)
                .await
                .ok()
                .and_then(|r| r.contents.into_iter().find_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => serde_json::from_str(&text).ok(), _ => None }))
                .unwrap_or_default();
            let view = crate::actions::knock_view_of(&k, title, secret.as_deref(), &members, chrono::Utc::now());
            assert!(view.checked && view.invited, "the seeded knock checks");
            knocks.push(view);
        }
        cancel.cancel();
        let entries = spaces.list().await;
        let personas: Vec<PersonaView> = identity
            .personas()
            .into_iter()
            .map(|p| PersonaView { id: p.id.clone(), name: p.name.clone(), usual: p.usual, rooms: entries.iter().filter(|e| identity.in_room(&e.id.id).map(|q| q.id == p.id).unwrap_or(p.usual)).map(|e| e.id.id.clone()).collect() })
            .collect();
        let you = identity.usual();

        let fixture = json!({
            "note": "Generated by `DIVERGE_EXPORT_PREVIEW=1 cargo test -p diverge-desktop export_preview_fixture`. Do not edit.",
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
            "tables": tables,
            "personas": personas,
            "you": { "name": you.name, "key": you.key },
            "trees": trees,
            "stats": stats,
            "mounts": daemon.creates().iter().map(|c| (c.name.clone(), AgentMounts::of_create(c).view())).collect::<BTreeMap<_, _>>(),
            "files": texts,
        });
        let json = serde_json::to_string_pretty(&fixture).unwrap();
        if std::env::var_os("DIVERGE_EXPORT_PREVIEW").is_some() {
            let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/preview/fixture.json");
            std::fs::create_dir_all(out.parent().unwrap()).unwrap();
            std::fs::write(out, json).unwrap();
        }
    }
}
