//! The tool on the other side.

use serde::{Deserialize, Serialize};

use crate::daemon::key;
use crate::shared::containers::request::Image;

/// A tool, as the daemon attests it on the `_meta` of everything its
/// server answers: the result of each of the four exchanges, each
/// tool of a tools list, each resource of a resources list, each
/// notification. Tagged `kind`, three kinds, flat — each with the
/// image the tool's container was made from and the template it was
/// made from. A created tool and a connected tool are one shape told
/// apart by the kind, the connected one naming the daemon it is on
/// besides. A created tool this daemon ran, and its image and its
/// template are the run request's. A connected tool another daemon
/// runs, and its image and its template are what that daemon told in
/// the `Connected` answer that opened the connect, once, as the scope
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
        /// The template it was made from, by id.
        template: String,
        /// Its number among all tools of the caller's ever made from
        /// that template, as its list item carries it.
        index: u64,
        /// Its name, as its create gave it, if it gave one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// A tool on record, registered from another daemon's tool, which
    /// runs it.
    Connected {
        /// The image it was made from, as the daemon that runs it told
        /// on the connect.
        image: Image,
        /// The template it was made from, by id, as the daemon that
        /// runs it told on the connect.
        template: String,
        /// Its number among all tools of the caller's ever registered
        /// from that daemon's tool, as its list item carries it.
        index: u64,
        /// Its name, as its register gave it, if it gave one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// The daemon that runs it, by the name this daemon's
        /// configuration gives it, as the register named it.
        daemon: String,
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
