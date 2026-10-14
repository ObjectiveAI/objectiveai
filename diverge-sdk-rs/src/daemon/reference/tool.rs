//! One tool of the caller's, named any of three ways.

use serde::{Deserialize, Serialize};

/// One tool of the caller's: by its name, by its template and its
/// index, or — a connected tool — by the daemon and the tool it
/// joined. See [`reference`](super) for which names what. Untagged
/// JSON, one object any way; an object with members of more than one
/// variant does not decode. A dependency tool is none of these: see
/// [`Dependency`](super::Dependency).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum Tool {
    /// By name: `{"name":…}`.
    Name {
        /// The tool's name, as its create or its register gave it. A
        /// tool given none is not reached this way.
        name: String,
    },
    /// By template and index: `{"template":…,"index":…}`.
    TemplateIndex {
        /// The template the tool was made from, by id.
        template: String,
        /// The tool's number among all tools of the caller's ever made
        /// from that template, as its list item carries it.
        index: u64,
    },
    /// By what it is registered from: `{"daemon":…,"account":…,"tool":…}`. A
    /// connected tool only, which has no template; the pair names it
    /// once and for all, as the register named it.
    Connected {
        /// The daemon the tool is on, by the name of its record.
        daemon: String,
        /// The account on that daemon the tool is reached as, by the
        /// name the record's accounts give it.
        account: String,
        /// The tool, as that daemon names it: any of these, read by
        /// that daemon and compared by this one.
        tool: Box<Tool>,
    },
}
