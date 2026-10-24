//! What the screen is handed by Rust: this app's own types, not the
//! daemon's — those the page reads with the SDK. Every type here writes
//! its own TypeScript (`cargo test` exports them to `src/bindings/`).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

// --- saved Views ---------------------------------------------------------

/// A saved View: a title and a log request, as the page shapes it with
/// the SDK (the daemon's `agents.logs` request less the agent). Rust
/// keeps it and reads nothing in it.
#[derive(Deserialize, Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct SavedView {
    pub id: String,
    pub title: String,
    #[ts(type = "unknown")]
    pub query: Value,
    pub saved: String,
}

// --- tabs (Rust-owned) ---------------------------------------------------

#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum TabKind {
    /// An agent, once and for all by its template and its index — the
    /// daemon's own key — with its name beside, when it has one.
    Agent {
        template: String,
        #[ts(type = "number")]
        index: u64,
        name: Option<String>,
    },
    NewAgent,
    Graph,
    Agents,
    Tools,
    Storage,
    Views,
    Settings,
}

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct TabView {
    pub key: String,
    pub tab: TabKind,
}

#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct TabsSnapshot {
    #[ts(type = "number")]
    pub generation: u64,
    pub tabs: Vec<TabView>,
    pub focused: Option<String>,
}

// --- the app itself --------------------------------------------------------

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct AppInfo {
    /// The folder the app keeps its files in.
    pub folder: String,
    /// Whether this copy of the app holds that folder. One that doesn't
    /// changes nothing in it, and the screens say so.
    pub folder_held: FolderHeld,
}

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum FolderHeld {
    /// This copy holds it, for as long as it runs.
    Yes,
    /// Another copy of the app holds it.
    InUse,
    /// The system couldn't say whether another copy does, in its own words.
    Unchecked { error: String },
}

impl From<&crate::store::Hold> for FolderHeld {
    fn from(h: &crate::store::Hold) -> Self {
        match h {
            crate::store::Hold::Held(_) => FolderHeld::Yes,
            crate::store::Hold::Elsewhere => FolderHeld::InUse,
            crate::store::Hold::Unchecked(error) => FolderHeld::Unchecked { error: error.clone() },
        }
    }
}

/// A file the app couldn't use since it started, and what it did about it.
#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct FileNoticeView {
    /// What kind of file it is: `views`, `machine names`, `theme`. The
    /// screen words it from `src/strings.ts`.
    pub kind: String,
    /// Whether it's the last good copy kept beside the file, not the file itself.
    pub last_good_copy: bool,
    /// Where it was.
    pub file: String,
    pub why: FileWhy,
    /// Where it is now, when it was set aside; none when it was left where it is.
    pub kept_as: Option<String>,
    /// What the system said, when it wouldn't read the file.
    pub error: Option<String>,
    pub carried_on: CarriedOnView,
}

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum FileWhy {
    /// It won't parse, or it's another kind of file.
    Damaged,
    /// It parses, but what it holds doesn't check.
    Refused,
    /// A newer version of the app wrote it.
    Newer,
    /// The system wouldn't open or read it this time.
    Unread,
}

#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum CarriedOnView {
    /// The last good copy, kept beside it.
    LastGood,
    /// Nothing: the app started that file empty.
    Empty,
    /// What the app had open: the file went bad after it was read.
    WhatItHad,
    /// Nothing, and nothing is written there until the app starts again.
    NothingThisLaunch,
}

impl FileNoticeView {
    /// A notice as the screen shows it.
    pub fn of(n: &crate::store::Notice) -> Self {
        use crate::store::{CarriedOn, Done, Why};
        let (why, kept_as, error) = match &n.done {
            Done::SetAside { kept_as, why } => (
                match why {
                    Why::Damaged => FileWhy::Damaged,
                    Why::Refused => FileWhy::Refused,
                    Why::Newer(_) => FileWhy::Newer,
                },
                Some(kept_as.display().to_string()),
                None,
            ),
            Done::LeftInPlace { error } => (FileWhy::Unread, None, Some(error.clone())),
        };
        FileNoticeView {
            kind: n.kind.to_owned(),
            last_good_copy: n.last_good_copy(),
            file: n.file.display().to_string(),
            why,
            kept_as,
            error,
            carried_on: match n.carried_on {
                CarriedOn::LastGood => CarriedOnView::LastGood,
                CarriedOn::Empty => CarriedOnView::Empty,
                CarriedOn::WhatItHad => CarriedOnView::WhatItHad,
                CarriedOn::NothingThisLaunch => CarriedOnView::NothingThisLaunch,
            },
        }
    }
}

/// One entry in the action registry: everything the page asks Rust for,
/// by name.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct ActionInfo {
    pub name: String,
    pub does: String,
}
