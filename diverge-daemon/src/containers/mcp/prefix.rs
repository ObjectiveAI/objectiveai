//! The prefixes an agent's served tools are named under: assigned
//! once per run, kept while served, never two the same.
//!
//! objectiveai's proxy named every server by a prefix free of the
//! separator, escalating on collision and never dropping a server;
//! the Eliza loop kept names stable across refreshes by giving a
//! newcomer a name avoiding those in use and freeing a leaver's. This
//! is both: a prefix is the serve-name folded to `[a-z0-9-]`,
//! escalated to `name-2`, `name-3`, … while taken, assigned when the
//! tool is first served in a run and kept until it leaves, so a
//! program that holds a name holds it for as long as the tool is
//! there. The exposed MCP name is `<prefix>_<name>`, and a call is
//! routed by its first `_`.

use std::collections::{HashMap, HashSet};

/// The separator between the prefix and the tool's own name: the one
/// character a prefix never holds.
pub const SEPARATOR: char = '_';

/// The prefixes of one run's served tools, by whatever the caller
/// keys a served tool by.
#[derive(Debug, Default)]
pub struct Registry<K> {
    assigned: HashMap<K, String>,
    taken: HashSet<String>,
}

impl<K: std::hash::Hash + Eq + Clone> Registry<K> {
    /// No tool served.
    pub fn new() -> Self {
        Registry {
            assigned: HashMap::new(),
            taken: HashSet::new(),
        }
    }

    /// The prefix the tool is served under, assigned now from
    /// `serve_name` if it has none: the folded name, or the first
    /// `name-N` not taken.
    pub fn assign(&mut self, key: K, serve_name: &str) -> String {
        if let Some(prefix) = self.assigned.get(&key) {
            return prefix.clone();
        }
        let base = fold(serve_name);
        let mut prefix = base.clone();
        let mut n = 2u64;
        while self.taken.contains(&prefix) {
            prefix = format!("{base}-{n}");
            n += 1;
        }
        self.taken.insert(prefix.clone());
        self.assigned.insert(key, prefix.clone());
        prefix
    }

    /// The prefix the tool is served under, if it is.
    pub fn prefix(&self, key: &K) -> Option<&str> {
        self.assigned.get(key).map(String::as_str)
    }

    /// The tool whose prefix this is, if any.
    pub fn owner(&self, prefix: &str) -> Option<&K> {
        self.assigned.iter().find(|(_, assigned)| assigned.as_str() == prefix).map(|(key, _)| key)
    }

    /// The tool leaves: its prefix is free for a newcomer.
    pub fn release(&mut self, key: &K) {
        if let Some(prefix) = self.assigned.remove(key) {
            self.taken.remove(&prefix);
        }
    }

    /// Every served tool and its prefix, in no order.
    pub fn all(&self) -> impl Iterator<Item = (&K, &str)> {
        self.assigned.iter().map(|(key, prefix)| (key, prefix.as_str()))
    }
}

/// The exposed name of a tool's MCP tool, resource or prompt.
pub fn exposed(prefix: &str, name: &str) -> String {
    format!("{prefix}{SEPARATOR}{name}")
}

/// An exposed name split at its first separator: the prefix and the
/// name the tool itself knows. None for a name with no separator.
pub fn split(exposed: &str) -> Option<(&str, &str)> {
    exposed.split_once(SEPARATOR)
}

/// A serve-name folded to a prefix: lowercase, every character outside
/// `[a-z0-9]` — the separator among them — made `-`, and runs of `-`
/// collapsed; `tool` for a name with nothing left.
pub fn fold(serve_name: &str) -> String {
    let mut folded = String::with_capacity(serve_name.len());
    let mut dash = true;
    for character in serve_name.chars() {
        let lower = character.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            folded.push(lower);
            dash = false;
        } else if !dash {
            folded.push('-');
            dash = true;
        }
    }
    let folded = folded.trim_end_matches('-').to_string();
    if folded.is_empty() { "tool".to_string() } else { folded }
}
