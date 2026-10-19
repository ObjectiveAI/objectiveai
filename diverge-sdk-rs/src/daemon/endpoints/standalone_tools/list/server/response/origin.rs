//! Where a tool comes from: made by this caller, or somebody else's
//! joined.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::logs::server::response::Provider;
use crate::daemon::endpoints::standalone_tools::templates::Mode;
use crate::daemon::reference;

/// The two ways a tool comes to be, and what the daemon knows of
/// each: a [`create`](crate::daemon::endpoints::standalone_tools::create) made it
/// from an image, or a
/// [`register`](crate::daemon::endpoints::standalone_tools::register) named a
/// tool another daemon holds. JSON-tagged by `kind`, `created` or
/// `connected`.
///
/// Either is a record, with the index its kind counts: the first made
/// the same way is `1`, each after is one more, and no number is given
/// twice, so the fixed part and the index name the tool once and for
/// all. A dependency tool — one running from a template an agent's
/// program declared — is no tool of this family: it is listed under
/// its template by the
/// [`dependency_tools`](crate::daemon::endpoints::dependency_tools) list, and
/// nowhere here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Origin {
    /// Made by this caller's create: the daemon runs its container.
    Created {
        /// The template it was made from, by id: the hash a
        /// [`templates::create`](crate::daemon::endpoints::standalone_tools::templates::create)
        /// answered.
        template: String,
        /// Its number among all tools of the caller's ever made from
        /// that template, deleted ones included.
        index: u64,
        /// How the tool is kept running, as its template's `mode` says:
        /// transient, run while used, or a service, kept while any
        /// agent is attached or any connect is open. See [`Mode`].
        mode: Mode,
        /// The provider the container runs on, if it runs, or last
        /// ran on; absent for a tool that has never run. See
        /// [`Provider`].
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider: Option<Provider>,
        /// The container's id while it runs, as the provider's run
        /// answered it. Absent while the container does not run.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    /// Another daemon's tool, registered by this caller: the daemon
    /// holds that daemon's connect scope on it while an attached agent
    /// uses it and for `idle_seconds` after, and never starts or stops
    /// it itself.
    Connected {
        /// The daemon the tool is on, by the name of its record, as
        /// the register named it.
        daemon: String,
        /// The account on that daemon the tool is reached as, as the
        /// register named it.
        account: String,
        /// The tool, as that daemon names it and as the register
        /// named it: see [`reference::Tool`].
        tool: reference::Tool,
        /// Its number among all tools of the caller's ever joined to
        /// that daemon's tool, deleted ones included.
        index: u64,
    },
}
