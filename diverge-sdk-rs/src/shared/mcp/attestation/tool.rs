//! The tool on the other side.

use serde::{Deserialize, Serialize};

use crate::daemon::key;
use crate::daemon::reference;
use crate::shared::containers::request::Image;

/// A tool, as the daemon attests it on the `_meta` of everything its
/// server answers: the result of each of the four exchanges, each
/// tool of a tools list, each resource of a resources list, each
/// notification. Tagged `kind`, three kinds, each with the image the
/// tool's container was made from, and nothing said twice: what the
/// tool's reference — the tool as a request of this daemon names it
/// once and for all — already says is said there alone. A created
/// tool this daemon ran: its image is the run request's, and its
/// reference, `{"template":…,"index":…}`, holds its template and its
/// index. A connected tool another daemon runs: its reference is the
/// daemon it is on and the tool as that daemon names it, and its
/// image and its template are what that daemon told in the
/// `Connected` answer that opened the connect, once, as the scope
/// opened — this daemon's stamp, not that daemon's, which is
/// replaced. A dependency tool this daemon ran from a template an
/// agent's program declared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Tool {
    /// A tool on record, made by a create from a template, run by
    /// this daemon.
    Created {
        /// The image it was made from.
        image: Image,
        /// Its name, as its create gave it, if it gave one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// The tool as a request of this daemon names it once and for
        /// all — `{"template":…,"index":…}`, the template it was made
        /// from and its number among all tools of the caller's ever
        /// made from it — a name being given or taken while this never
        /// changes. The template and the index are here and nowhere
        /// else beside.
        tool: reference::Tool,
    },
    /// A tool on record, registered from another daemon's tool, which
    /// runs it.
    Connected {
        /// The image it was made from, as the daemon that runs it told
        /// on the connect.
        image: Image,
        /// The template it was made from, by id, as the daemon that
        /// runs it told on the connect: that daemon's, which the
        /// reference does not name.
        template: String,
        /// Its number among all tools of the caller's ever registered
        /// from that daemon's tool, as its list item carries it.
        index: u64,
        /// Its name, as its register gave it, if it gave one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// The daemon that runs it, by the name of its record, as the
        /// register named it.
        daemon: String,
        /// The tool, as that daemon names it, as the register named
        /// it. With the daemon, the tool as a request of this daemon
        /// names it once and for all — `{"daemon":…,"tool":…}` — a
        /// name being given or taken while this never changes.
        tool: reference::Tool,
    },
    /// A dependency tool, run by this daemon from a template an
    /// agent's program declared.
    Dependency {
        /// The image it was made from.
        image: Image,
        /// The dependency template it runs from, by id.
        template: String,
        /// Whose it is: see [`key::Owner`].
        owner: key::Owner,
    },
}

impl Tool {
    /// The image the tool's container was made from: every kind's.
    pub fn image(&self) -> &Image {
        match self {
            Tool::Created { image, .. } | Tool::Connected { image, .. } | Tool::Dependency { image, .. } => image,
        }
    }
}
