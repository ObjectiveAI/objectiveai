//! The tool on the other side.

use serde::{Deserialize, Serialize};

use crate::daemon::key;
use crate::daemon::reference;
use crate::shared::containers::request::Image;

/// A tool, as the daemon attests it on the `_meta` of everything its
/// server answers: the result of each of the four exchanges, each
/// tool of a tools list, each resource of a resources list, each
/// notification. Tagged `kind`, three kinds — and the image a member
/// of two of them. A created tool and a dependency tool the daemon
/// ran itself, from an image it named; a connected tool runs on
/// another daemon, which attested it, and this daemon saw neither the
/// image nor the container. The kinds are the daemon's own
/// [`key::Tool`] laid flat, the image beside: [`key`](Self::key)
/// gives that key back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Tool {
    /// A tool on record, made by a create from a template, run by
    /// this daemon.
    Created {
        /// The image it was made from.
        image: Image,
        /// The template, by id.
        template: String,
        /// Its number among all tools of the caller's ever made from
        /// that template, as its list item carries it.
        index: u64,
        /// Its name, as its create gave it, if it gave one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// A tool on record, registered from another daemon's tool, which
    /// runs it: no image, since this daemon never saw one.
    Connected {
        /// The daemon the tool is on, by the name of its record.
        daemon: String,
        /// The tool, as that daemon names it: see
        /// [`reference::Tool`].
        tool: Box<reference::Tool>,
        /// Its number among all tools of the caller's ever registered
        /// from that daemon's tool, as its list item carries it.
        index: u64,
        /// Its name, as its connect gave it, if it gave one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
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
    /// The image the tool was made from: every kind's but a connected
    /// tool's, which is the one kind without.
    pub fn image(&self) -> Option<&Image> {
        match self {
            Tool::Created { image, .. } | Tool::Dependency { image, .. } => Some(image),
            Tool::Connected { .. } => None,
        }
    }

    /// The tool as the daemon keys it: the same members, without the
    /// image.
    pub fn key(&self) -> key::Tool {
        match self {
            Tool::Created {
                template,
                index,
                name,
                ..
            } => key::Tool::Record {
                origin: key::Origin::Created {
                    template: template.clone(),
                },
                index: *index,
                name: name.clone(),
            },
            Tool::Connected {
                daemon,
                tool,
                index,
                name,
            } => key::Tool::Record {
                origin: key::Origin::Connected {
                    daemon: daemon.clone(),
                    tool: tool.clone(),
                },
                index: *index,
                name: name.clone(),
            },
            Tool::Dependency {
                template, owner, ..
            } => key::Tool::Dependency {
                template: template.clone(),
                owner: owner.clone(),
            },
        }
    }
}
