//! The tools one run is served.

use std::collections::HashMap;
use std::sync::Arc;

use rmcp::model::ServerNotification;
use tokio::sync::broadcast;

use super::Registry;
use crate::containers::ToolRun;
use crate::store::ToolId;
use crate::store::tools::Tool;

/// One served tool: its record until its container is needed, its
/// run once it is.
pub enum Entry {
    /// Not running; started on the loop's begin or the first ask.
    Idle(Tool),
    /// Running, or joined.
    Running(Arc<ToolRun>),
}

/// The tools one run is served, each under its prefix, and the word
/// to the program when the set changes.
pub struct Served {
    registry: Registry<ToolId>,
    entries: HashMap<ToolId, Entry>,
    changes: broadcast::Sender<ServerNotification>,
}

impl Default for Served {
    fn default() -> Self {
        Served::new()
    }
}

impl Served {
    /// Nothing served.
    pub fn new() -> Self {
        Served {
            registry: Registry::new(),
            entries: HashMap::new(),
            changes: broadcast::channel(64).0,
        }
    }

    /// Serve the tool under `serve_name`, idle; the prefix it gets,
    /// kept until it leaves. One served already keeps its prefix.
    pub fn insert(&mut self, tool: Tool, serve_name: &str) -> String {
        let id = tool.id;
        let prefix = self.registry.assign(id, serve_name);
        self.entries.entry(id).or_insert(Entry::Idle(tool));
        self.changed();
        prefix
    }

    /// The tool's container is running.
    pub fn running(&mut self, id: ToolId, run: Arc<ToolRun>) {
        if self.entries.contains_key(&id) {
            self.entries.insert(id, Entry::Running(run));
        }
    }

    /// The tool is idle again: its run ended.
    pub fn idle(&mut self, id: ToolId, tool: Tool) {
        if self.entries.contains_key(&id) {
            self.entries.insert(id, Entry::Idle(tool));
        }
    }

    /// The tool leaves: its prefix is free.
    pub fn remove(&mut self, id: ToolId) -> Option<Entry> {
        let entry = self.entries.remove(&id);
        if entry.is_some() {
            self.registry.release(&id);
            self.changed();
        }
        entry
    }

    /// The entry for the tool, if served.
    pub fn entry(&self, id: ToolId) -> Option<&Entry> {
        self.entries.get(&id)
    }

    /// Every served tool, by id, in no order.
    pub fn ids(&self) -> Vec<ToolId> {
        self.entries.keys().copied().collect()
    }

    /// Every running served tool.
    pub fn runs(&self) -> Vec<Arc<ToolRun>> {
        self.entries
            .values()
            .filter_map(|entry| match entry {
                Entry::Running(run) => Some(Arc::clone(run)),
                Entry::Idle(_) => None,
            })
            .collect()
    }

    /// The tool's prefix.
    pub fn prefix(&self, id: ToolId) -> Option<String> {
        self.registry.prefix(&id).map(str::to_string)
    }

    /// The tool a prefix belongs to.
    pub fn owner(&self, prefix: &str) -> Option<ToolId> {
        self.registry.owner(prefix).copied()
    }

    /// Where the program hears that the set changed.
    pub fn subscribe(&self) -> broadcast::Receiver<ServerNotification> {
        self.changes.subscribe()
    }

    /// The set changed: tell every listener.
    fn changed(&self) {
        let _ = self.changes.send(ServerNotification::ToolListChangedNotification(Default::default()));
    }
}
