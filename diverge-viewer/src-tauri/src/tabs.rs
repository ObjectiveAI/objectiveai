//! Tabs are Rust's, not the page's — the viewer's pattern, rebuilt small.
//! One ordered list, a focused key, and a generation that goes up on every
//! change; the whole snapshot is broadcast each time, so a page (or, later,
//! an agent) never has to reconcile deltas.

use crate::view::{TabKind, TabView, TabsSnapshot};

pub struct Tabs {
    generation: u64,
    tabs: Vec<TabView>,
    focused: Option<String>,
}

/// The tabs a viewer starts with: Graph, open and focused.
impl Default for Tabs {
    fn default() -> Self {
        let mut tabs = Tabs { generation: 0, tabs: Vec::new(), focused: None };
        tabs.open(TabKind::Graph);
        tabs
    }
}

pub fn key_of(tab: &TabKind) -> String {
    match tab {
        TabKind::Agent { template, index, .. } => format!("agent:{template}:{index}"),
        TabKind::NewAgent => "new-agent".into(),
        TabKind::Graph => "graph".into(),
        TabKind::Agents => "agents".into(),
        TabKind::Tools => "tools".into(),
        TabKind::Storage => "storage".into(),
        TabKind::Views => "views".into(),
        TabKind::Settings => "settings".into(),
    }
}

impl Tabs {
    pub fn snapshot(&self) -> TabsSnapshot {
        TabsSnapshot { generation: self.generation, tabs: self.tabs.clone(), focused: self.focused.clone() }
    }

    /// Open, or focus if already open: one tab per key.
    pub fn open(&mut self, tab: TabKind) -> TabsSnapshot {
        let key = key_of(&tab);
        if !self.tabs.iter().any(|t| t.key == key) {
            self.tabs.push(TabView { key: key.clone(), tab });
        }
        self.focused = Some(key);
        self.generation += 1;
        self.snapshot()
    }

    pub fn close(&mut self, key: &str) -> TabsSnapshot {
        if let Some(at) = self.tabs.iter().position(|t| t.key == key) {
            self.tabs.remove(at);
            if self.focused.as_deref() == Some(key) {
                self.focused = self.tabs.get(at.min(self.tabs.len().saturating_sub(1))).map(|t| t.key.clone());
            }
            self.generation += 1;
        }
        self.snapshot()
    }
}
