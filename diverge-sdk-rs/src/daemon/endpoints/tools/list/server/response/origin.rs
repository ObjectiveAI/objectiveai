//! Where a tool comes from: made by this caller, somebody else's
//! joined, or deployed for an agent.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::logs::server::response::Provider;
use crate::daemon::key;
use crate::daemon::reference;
use crate::shared::containers::dependencies::Template;

/// The three ways a tool comes to be, and what the daemon knows of
/// each: a [`create`](crate::daemon::endpoints::tools::create) made it
/// from an image, a
/// [`connect`](crate::daemon::endpoints::tools::connect) named a
/// tool another daemon holds, or an agent's program declared it
/// as a dependency and the daemon deployed it when the agent's
/// container started. JSON-tagged by `kind`, `created`, `connected`
/// or `dependency`.
///
/// A created or a connected tool is a record, with the index its
/// kind counts: the first made the same way is `1`, each after is one
/// more, and no number is given twice, so the fixed part and the
/// index name the tool once and for all. A dependency tool is no
/// record: it is listed while its agent's container runs and not
/// after, named once and for all by its agent and its template's id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Origin {
    /// Made by this caller's create: the daemon runs its container.
    Created {
        /// The template it was made from, by id: the hash a
        /// [`templates::create`](crate::daemon::endpoints::tools::templates::create)
        /// answered.
        template: String,
        /// Its number among all tools of the caller's ever made from
        /// that template, deleted ones included.
        index: u64,
        /// The provider the container runs on, if it runs, or last
        /// ran on; absent for a tool that has never run. See
        /// [`Provider`].
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider: Option<Provider>,
        /// The container's id while it runs, as the provider's run
        /// answered it: what an
        /// [`expose`](crate::daemon::endpoints::tools::expose) hands,
        /// with the provider and an authorization, to the daemon it
        /// lets connect. Absent while the container does not run.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    /// Another daemon's tool, joined by this caller's connect: the
    /// daemon holds that daemon's expose and a connect scope on the
    /// container while an attached agent is active, and never starts
    /// or stops it itself.
    Connected {
        /// The daemon the tool is on, by the name of its record, as
        /// the connect named it.
        daemon: String,
        /// The tool, as that daemon names it and as the connect named
        /// it: see [`reference::Tool`].
        tool: reference::Tool,
        /// Its number among all tools of the caller's ever joined to
        /// that daemon's tool, deleted ones included.
        index: u64,
    },
    /// Deployed for an agent, from the dependency tool template the
    /// agent's program declared: the daemon runs its container for as
    /// long as the agent's runs, and lists it only then.
    Dependency {
        /// The agent it was deployed for: see [`key::Agent`].
        agent: key::Agent,
        /// The dependency tool template it was deployed from, by id:
        /// the lowercase hexadecimal SHA-256 of the template's
        /// canonical bytes, unique among that agent's dependencies.
        /// The agent calls the tool's MCP tools by its first eight
        /// characters, as a prefix.
        template: String,
        /// The template whole, as the program declared it: its image,
        /// its limits, its arguments, which database scope it gets,
        /// the agent's paths served into it, and the grants its
        /// requests are judged by. See [`Template`].
        declared: Template,
        /// The provider the container runs on. See [`Provider`].
        provider: Provider,
        /// The container's id, as the provider's run answered it.
        id: String,
    },
}
