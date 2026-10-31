//! Whose log: an agent's or a tool's, and what its entries are.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::agent_log as agent_log;
use diverge_sdk::daemon::command_log;
use diverge_sdk::daemon::tool_log;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::commands::AdminKey;
use crate::containers::ToolKey;
use crate::store::{AccountId, AgentId};

/// One entry of a log, as the wire sends it: an item with its place
/// and its time.
pub trait Entry: Serialize + DeserializeOwned {
    /// What the entry holds.
    type Item;

    /// The entry numbered `logs_index`, kept at `created`.
    fn wrap(logs_index: u64, created: DateTime<Utc>, item: Self::Item) -> Self;

    /// When it was kept.
    fn created(&self) -> DateTime<Utc>;
}

impl Entry for agent_log::ItemWrapper {
    type Item = agent_log::Item;

    fn wrap(logs_index: u64, created: DateTime<Utc>, item: Self::Item) -> Self {
        agent_log::ItemWrapper { logs_index, created, item }
    }

    fn created(&self) -> DateTime<Utc> {
        self.created
    }
}

impl Entry for command_log::ItemWrapper {
    type Item = command_log::Item;

    fn wrap(logs_index: u64, created: DateTime<Utc>, item: Self::Item) -> Self {
        command_log::ItemWrapper { logs_index, created, item }
    }

    fn created(&self) -> DateTime<Utc> {
        self.created
    }
}

impl Entry for tool_log::ItemWrapper {
    type Item = tool_log::Item;

    fn wrap(logs_index: u64, created: DateTime<Utc>, item: Self::Item) -> Self {
        tool_log::ItemWrapper { logs_index, created, item }
    }

    fn created(&self) -> DateTime<Utc> {
        self.created
    }
}

/// Whose log: where it lies under the daemon's `<dir>`, and what its
/// entries are.
pub trait Book: Copy {
    /// What one entry is.
    type Entry: Entry;

    /// The log's directory under `root`, the daemon's `<dir>`.
    fn dir(self, root: &Path) -> PathBuf;
}

/// An agent's log: `<dir>/agents/<id>/`.
impl Book for AgentId {
    type Entry = agent_log::ItemWrapper;

    fn dir(self, root: &Path) -> PathBuf {
        root.join("agents").join(self.0.to_string())
    }
}

/// A tool's log: `<dir>/tools/<id>/` for a tool on record,
/// `<dir>/dependency_tools/<id>/` for a dependency tool.
impl Book for ToolKey {
    type Entry = tool_log::ItemWrapper;

    fn dir(self, root: &Path) -> PathBuf {
        match self {
            ToolKey::Record(id) => root.join("tools").join(id.0.to_string()),
            ToolKey::Dependency(id) => root.join("dependency_tools").join(id.0.to_string()),
        }
    }
}

/// An account's command log: `<dir>/accounts/<id>/`.
impl Book for AccountId {
    type Entry = command_log::ItemWrapper;

    fn dir(self, root: &Path) -> PathBuf {
        root.join("accounts").join(self.0.to_string())
    }
}

/// An administrator's command log: `<dir>/admins/<key in hex>/`.
impl Book for AdminKey {
    type Entry = command_log::ItemWrapper;

    fn dir(self, root: &Path) -> PathBuf {
        root.join("admins").join(hex::encode(self.0))
    }
}
