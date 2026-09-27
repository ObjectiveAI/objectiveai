//! Diverge — the desktop app, draft one.
//!
//! Rust owns the daemon, its address and its credentials; the page gets
//! commands (see [`actions`]) and one channel per stream. Today the daemon
//! is [`daemon::stub::StubDaemon`]; see `diverge-desktop/CLAUDE.md`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod catalog;
mod daemon;
mod machines;
mod door;
mod identity;
mod preview;
mod reporter;
mod spaces;
mod tabs;
mod view;

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{Emitter, Manager};

use actions::AppState;
use daemon::stub::StubDaemon;
use spaces::stub::StubSpaces;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // The menu: the OS's own Edit items (text fields need them), and
            // the app's shortcuts. ⌘W closes a tab, not the window.
            let app_menu = SubmenuBuilder::new(app, "Diverge").about(None).separator().services().separator().hide().hide_others().show_all().separator().quit().build()?;
            let edit = SubmenuBuilder::new(app, "Edit").undo().redo().separator().cut().copy().paste().select_all().build()?;
            let new_agent = MenuItemBuilder::with_id("new-agent", "New Agent").accelerator("CmdOrCtrl+N").build(app)?;
            let close_tab = MenuItemBuilder::with_id("close-tab", "Close Tab").accelerator("CmdOrCtrl+W").build(app)?;
            let home = MenuItemBuilder::with_id("go-home", "Home").accelerator("CmdOrCtrl+1").build(app)?;
            let inbox = MenuItemBuilder::with_id("go-inbox", "Inbox").accelerator("CmdOrCtrl+2").build(app)?;
            let go = SubmenuBuilder::new(app, "Go").item(&home).item(&inbox).separator().item(&new_agent).separator().item(&close_tab).build()?;
            let window = SubmenuBuilder::new(app, "Window").minimize().maximize().separator().fullscreen().build()?;
            let menu = MenuBuilder::new(app).items(&[&app_menu, &edit, &go, &window]).build()?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, event| {
                let _ = app.emit("menu://action", event.id().as_ref().to_owned());
            });

            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;
            let host = data.join("stand-in-host");
            // Your keys: made on first run, under your usual name (your Mac's
            // account name until you rename it), kept in one owner-only file.
            let usual_name = std::env::var("USER").ok().filter(|u| !u.is_empty()).unwrap_or_else(|| "you".into());
            let identity = Arc::new(identity::Identity::open(data.join("identity.json"), &usual_name));
            let (daemon, spaces, door) = tauri::async_runtime::block_on({
                let host = host.clone();
                let identity = identity.clone();
                let allowances = data.join("allowances.json");
                async move {
                    let daemon = StubDaemon::new(host.clone());
                    let spaces: Arc<dyn spaces::Spaces> = Arc::new(StubSpaces::new(identity.clone(), host.join("tables")));
                    let door = Arc::new(door::Door::new(spaces.clone(), identity, Some(allowances)));
                    daemon.set_door(door.clone());
                    (daemon, spaces, door)
                }
            });
            let views_file = data.join("views.json");
            let machine_names_file = data.join("machine_names.json");
            let machine_names = std::fs::read_to_string(&machine_names_file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
            let views = std::fs::read_to_string(&views_file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
            // What the app last stated each agent mounts. On a first run with
            // the stand-in, its seeded agents count as made here.
            let agent_mounts_file = data.join("agent_mounts.json");
            let agent_mounts: HashMap<String, view::AgentMounts> = match std::fs::read_to_string(&agent_mounts_file).ok().and_then(|s| serde_json::from_str(&s).ok()) {
                Some(known) => known,
                None => daemon.creates().iter().map(|c| (c.name.clone(), view::AgentMounts::of_create(c))).collect(),
            };
            // The stand-in answers both seams: it knows every agent's mounts,
            // so it keeps every machine's holds.
            let daemon = Arc::new(daemon);
            let machines: Arc<dyn machines::Machines> = daemon.clone();
            let daemon: Arc<dyn daemon::Daemon> = daemon;
            reporter::spawn(daemon.clone(), spaces.clone(), identity.clone());
            app.manage(AppState {
                stand_in_host: Some(host.clone()),
                daemon,
                identity,
                machines,
                spaces,
                door,
                scopes: Mutex::new(HashMap::new()),
                next_scope: AtomicU64::new(1),
                tabs: Mutex::new(tabs::Tabs::default()),
                views: Mutex::new(views),
                views_file,
                machine_names: Mutex::new(machine_names),
                machine_names_file,
                agent_mounts: Mutex::new(agent_mounts),
                agent_mounts_file,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            actions::agents_list,
            actions::agents_create,
            actions::agents_delete,
            actions::agents_edit,
            actions::agents_mounts,
            actions::agents_message,
            actions::agents_message_take_back,
            actions::logs_open,
            actions::scope_close,
            actions::volumes_list,
            actions::volumes_stat,
            actions::volumes_tree,
            actions::volumes_read,
            actions::volumes_write,
            actions::volumes_room,
            actions::volumes_create,
            actions::volumes_room_for,
            actions::volumes_edit,
            actions::volumes_delete,
            actions::home_feed,
            actions::people_list,
            actions::profile_get,
            actions::spaces_home,
            actions::spaces_list,
            actions::spaces_get,
            actions::spaces_feed,
            actions::spaces_call,
            actions::spaces_watch,
            actions::spaces_host,
            actions::spaces_join,
            actions::spaces_leave,
            actions::spaces_invite,
            actions::knocks_watch,
            actions::knocks_answer,
            actions::spaces_door,
            actions::asks_send,
            actions::table_tree,
            actions::table_read,
            actions::table_write,
            actions::table_transfer,
            actions::personas_list,
            actions::persona_rename,
            actions::allowance_get,
            actions::allowance_set,
            actions::cards_watch,
            actions::cards_answer,
            actions::door_tools,
            actions::machines_list,
            actions::machines_add,
            actions::machines_remove,
            actions::machines_rename,
            actions::machines_names,
            actions::views_list,
            actions::views_save,
            actions::views_delete,
            actions::catalog_images,
            actions::catalog_check,
            actions::tabs_snapshot,
            actions::tabs_open,
            actions::tabs_close,
            actions::tabs_focus,
            actions::actions_list,
            actions::app_info,
        ])
        .run(tauri::generate_context!())
        .expect("the app could not start");
}
