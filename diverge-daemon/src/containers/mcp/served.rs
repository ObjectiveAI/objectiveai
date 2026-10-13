//! The tools one run is served.

use std::collections::HashMap;
use std::sync::Arc;

use rmcp::model::ServerNotification;
use tokio::sync::broadcast;

use super::Registry;
use crate::containers::{ToolKey, ToolRun};
use crate::store::DependencyToolId;
use crate::store::tools::Tool;

/// One served tool: an attached record, with its run while it has
/// one; a transient dependency tool, which runs for as long as it is
/// served; or a service, by its row, whose run — up or not, and a
/// new one after every end — is the live set's to find.
pub enum Entry {
    /// A tool attached to the agent: its record, and its run once
    /// its container is needed — on the loop's begin or the first ask
    /// — until the loop ends.
    Attached {
        /// The record.
        tool: Tool,
        /// The run, while there is one.
        run: Option<Arc<ToolRun>>,
    },
    /// A transient dependency tool deployed for the agent: running for
    /// the agent's run.
    Transient(Arc<ToolRun>),
    /// A service the agent is attached to: kept by the daemon, its
    /// run looked up when it is called.
    Service {
        /// The row.
        id: DependencyToolId,
        /// The template it was made from, by id.
        template: String,
    },
}

/// The tools one run is served, each under its prefix, and the word
/// to the program when the set changes.
pub struct Served {
    registry: Registry<ToolKey>,
    entries: HashMap<ToolKey, Entry>,
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

    /// Serve the attached tool under `serve_name`, idle; the prefix it
    /// gets, kept until it leaves. One served already keeps its
    /// prefix and its entry.
    pub fn insert(&mut self, tool: Tool, serve_name: &str) -> String {
        let key = ToolKey::Record(tool.id);
        let prefix = self.registry.assign(key, serve_name);
        self.entries.entry(key).or_insert(Entry::Attached { tool, run: None });
        self.changed();
        prefix
    }

    /// Serve the transient dependency tool under `serve_name`, its
    /// template's id short; the prefix it gets.
    pub fn insert_transient(&mut self, run: Arc<ToolRun>, serve_name: &str) -> String {
        let key = run.id;
        let prefix = self.registry.assign(key, serve_name);
        self.entries.insert(key, Entry::Transient(run));
        self.changed();
        prefix
    }

    /// Serve the service under `serve_name`, its template's id short;
    /// the prefix it gets.
    pub fn insert_service(&mut self, id: DependencyToolId, template: &str, serve_name: &str) -> String {
        let key = ToolKey::Dependency(id);
        let prefix = self.registry.assign(key, serve_name);
        self.entries.insert(
            key,
            Entry::Service {
                id,
                template: template.to_string(),
            },
        );
        self.changed();
        prefix
    }

    /// The attached tool's container is running.
    pub fn running(&mut self, key: ToolKey, run: Arc<ToolRun>) {
        if let Some(Entry::Attached { run: held, .. }) = self.entries.get_mut(&key) {
            *held = Some(run);
        }
    }

    /// The attached tool is idle again: its run ended.
    pub fn idle(&mut self, key: ToolKey) {
        if let Some(Entry::Attached { run, .. }) = self.entries.get_mut(&key) {
            *run = None;
        }
    }

    /// The tool leaves: its prefix is free.
    pub fn remove(&mut self, key: ToolKey) -> Option<Entry> {
        let entry = self.entries.remove(&key);
        if entry.is_some() {
            self.registry.release(&key);
            self.changed();
        }
        entry
    }

    /// The entry for the tool, if served.
    pub fn entry(&self, key: ToolKey) -> Option<&Entry> {
        self.entries.get(&key)
    }

    /// Every served tool, by key, in no order.
    pub fn keys(&self) -> Vec<ToolKey> {
        self.entries.keys().copied().collect()
    }

    /// Every running served tool this set holds the run of: attached
    /// ones with a run, and every transient dependency tool. A service's
    /// run is the live set's; see [`services`](Self::services).
    pub fn runs(&self) -> Vec<Arc<ToolRun>> {
        self.entries
            .values()
            .filter_map(|entry| match entry {
                Entry::Attached { run, .. } => run.clone(),
                Entry::Transient(run) => Some(Arc::clone(run)),
                Entry::Service { .. } => None,
            })
            .collect()
    }

    /// Every attached tool whose container runs, by key, in no order:
    /// what a loop's end releases.
    pub fn attached_running(&self) -> Vec<ToolKey> {
        self.entries
            .iter()
            .filter_map(|(key, entry)| match entry {
                Entry::Attached { run: Some(_), .. } => Some(*key),
                _ => None,
            })
            .collect()
    }

    /// Every transient dependency tool, in no order: what is stopped
    /// with the agent's run.
    pub fn transients(&self) -> Vec<Arc<ToolRun>> {
        self.entries
            .values()
            .filter_map(|entry| match entry {
                Entry::Transient(run) => Some(Arc::clone(run)),
                Entry::Attached { .. } | Entry::Service { .. } => None,
            })
            .collect()
    }

    /// Every service, by id, in no order.
    pub fn services(&self) -> Vec<DependencyToolId> {
        self.entries
            .values()
            .filter_map(|entry| match entry {
                Entry::Service { id, .. } => Some(*id),
                Entry::Attached { .. } | Entry::Transient(_) => None,
            })
            .collect()
    }

    /// The tool's prefix.
    pub fn prefix(&self, key: ToolKey) -> Option<String> {
        self.registry.prefix(&key).map(str::to_string)
    }

    /// The tool a prefix belongs to.
    pub fn owner(&self, prefix: &str) -> Option<ToolKey> {
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
