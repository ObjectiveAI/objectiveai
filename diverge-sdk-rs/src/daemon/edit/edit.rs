//! The members an agent's edit and a tool's edit share.

use serde::{Deserialize, Serialize};

use crate::daemon::daemon_tools::{
    AgentsTag, AgentsTemplatesTag, AgentsTemplatesUntag, AgentsUntag, Edge, Held, Reach, Switch, ToolsAttach,
    ToolsDetach, ToolsTag, ToolsTemplatesTag, ToolsTemplatesUntag, ToolsUntag,
};
use crate::daemon::endpoints::agents::create::client::request::{FuseMount, VolumeMount};
use crate::daemon::endpoints::{agents, tools};
use crate::daemon::reference;
use super::Change;

/// Everything about a container that changes after its create, every
/// member an optional [`Change`]. A member absent leaves that of the
/// container as it is; `delete` takes it away; `set` replaces it whole
/// — a list of mounts is the new list entire, a tool's reach the new
/// reach, the name the new name — and nothing is merged. A request with
/// every member absent changes nothing and is not a failure. What is
/// not here does not change: the template, the provider pin, the index,
/// who made it.
///
/// # What waits for the container to be inactive
///
/// The mounts: a request that names any of the three mount lists is
/// refused while the container is active, and left as it is. The name,
/// the daemon's tools and the deployer change live.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edit {
    /// The name, unique among the caller's agents or tools as the
    /// container is one or the other; a name another holds is the
    /// edit's `InUse`, and nothing changes. Absent, as it is; `delete`,
    /// the container has no name, and is reached once and for all only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Change<String>>,
    /// Volumes of the provider the create pinned the container to, as
    /// it is to mount them: see [`VolumeMount`]. The new list whole; a
    /// container pinned to no provider mounts no volume, and a request
    /// naming one for it is the edit's error. Ordered, and applied in
    /// order; no mount's path, in any list, is a prefix of another's.
    /// Absent, as it is; `delete`, no volume mounted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_mounts: Option<Change<Vec<VolumeMount>>>,
    /// Files of providers' volumes served live across the daemon,
    /// mounted one each over FUSE, as the container is to mount them:
    /// see [`FuseMount`], and the create's
    /// [`fuse_file_mounts`](crate::daemon::create::Inner::fuse_file_mounts).
    /// The new list whole. Absent, as it is; `delete`, none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuse_file_mounts: Option<Change<Vec<FuseMount>>>,
    /// Directories of providers' volumes served live across the daemon,
    /// mounted one each over FUSE, as the container is to mount them:
    /// see [`FuseMount`], and the create's
    /// [`fuse_directory_mounts`](crate::daemon::create::Inner::fuse_directory_mounts).
    /// The new list whole. Absent, as it is; `delete`, none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuse_directory_mounts: Option<Change<Vec<FuseMount>>>,
    /// Whether the container may name itself: see the create's
    /// [`self`](crate::daemon::create::Inner::itself). Absent, as it
    /// is; `delete`, `false`.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "self")]
    pub itself: Option<Change<bool>>,
    /// The tool that lists agents; `only`, the agents the filter
    /// passes, and its own list requests narrow within them. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_list: Option<Change<Reach<agents::list::client::request::Filter>>>,
    /// The tool that gets one agent as a list would report it; `only`,
    /// the agents the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_get: Option<Change<Reach<agents::list::client::request::Filter>>>,
    /// The tool that messages agents; `only`, the agents the filter
    /// passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_message: Option<Change<Reach<agents::list::client::request::Filter>>>,
    /// The tool that reads agents' logs; `only`, the agents the filter
    /// passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_logs: Option<Change<Reach<agents::list::client::request::Filter>>>,
    /// The tool that makes agents; `only`, from the agent templates the
    /// filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_create: Option<Change<Reach<agents::templates::list::client::request::Filter>>>,
    /// The tool that deletes agents; `only`, the agents the filter
    /// passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_delete: Option<Change<Reach<agents::list::client::request::Filter>>>,
    /// The tool that edits agents; `only`, the agents the filter
    /// passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_edit: Option<Change<Reach<agents::list::client::request::Filter>>>,
    /// The tool that tags agents: `disabled`, or `only` which agents
    /// and with which tags, each side `any` or `only`: see
    /// [`AgentsTag`].
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_tag: Option<Change<Held<AgentsTag>>>,
    /// The tool that untags agents: `disabled`, or `only` which agents
    /// and of which tags, each side `any` or `only`: see
    /// [`AgentsUntag`].
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_untag: Option<Change<Held<AgentsUntag>>>,
    /// The tool that makes agent templates: see [`Switch`].
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_templates_create: Option<Change<Switch>>,
    /// The tool that lists agent templates; `only`, the templates the
    /// filter passes, and its own list requests narrow within them. See
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_templates_list: Option<Change<Reach<agents::templates::list::client::request::Filter>>>,
    /// The tool that gets one agent template by id; `only`, the
    /// templates the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_templates_get: Option<Change<Reach<agents::templates::list::client::request::Filter>>>,
    /// The tool that deletes agent templates; `only`, the templates the
    /// filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_templates_delete: Option<Change<Reach<agents::templates::list::client::request::Filter>>>,
    /// The tool that tags agent templates: `disabled`, or `only` which
    /// templates and with which tags, each side `any` or `only`: see
    /// [`AgentsTemplatesTag`].
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_templates_tag: Option<Change<Held<AgentsTemplatesTag>>>,
    /// The tool that untags agent templates: `disabled`, or `only`
    /// which templates and of which tags, each side `any` or `only`:
    /// see [`AgentsTemplatesUntag`].
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_templates_untag: Option<Change<Held<AgentsTemplatesUntag>>>,
    /// The tool that lists tools; `only`, the tools the filter passes,
    /// and its own list requests narrow within them. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_list: Option<Change<Reach<tools::list::client::request::Filter>>>,
    /// The tool that gets one tool as a list would report it; `only`,
    /// the tools the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_get: Option<Change<Reach<tools::list::client::request::Filter>>>,
    /// The tool that makes tools; `only`, from the tool templates the
    /// filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::templates::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_create: Option<Change<Reach<tools::templates::list::client::request::Filter>>>,
    /// The tool that edits tools; `only`, the tools the filter passes.
    /// See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_edit: Option<Change<Reach<tools::list::client::request::Filter>>>,
    /// The tool that deletes tools; `only`, the tools the filter
    /// passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_delete: Option<Change<Reach<tools::list::client::request::Filter>>>,
    /// The tool that attaches tools to agents; `only`, what it may join
    /// to what: see [`ToolsAttach`].
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_attach: Option<Change<Reach<ToolsAttach>>>,
    /// The tool that detaches tools from agents; `only`, what it may
    /// part from what: see [`ToolsDetach`].
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_detach: Option<Change<Reach<ToolsDetach>>>,
    /// The tool that tags tools: `disabled`, or `only` which tools and
    /// with which tags, each side `any` or `only`: see [`ToolsTag`].
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_tag: Option<Change<Held<ToolsTag>>>,
    /// The tool that untags tools: `disabled`, or `only` which tools
    /// and of which tags, each side `any` or `only`: see
    /// [`ToolsUntag`].
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_untag: Option<Change<Held<ToolsUntag>>>,
    /// The tool that routes a dependency position to a tool; `only`, to
    /// the tools the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_routes_set: Option<Change<Reach<tools::list::client::request::Filter>>>,
    /// The tool that takes a route up; `only`, the routes the filter
    /// passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::routes::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_routes_delete: Option<Change<Reach<tools::routes::list::client::request::Filter>>>,
    /// The tool that lists routes; `only`, the routes the filter
    /// passes, and its own list requests narrow within them. See
    /// [`Filter`](crate::daemon::endpoints::tools::routes::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_routes_list: Option<Change<Reach<tools::routes::list::client::request::Filter>>>,
    /// The tool that makes tool templates: see [`Switch`].
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_templates_create: Option<Change<Switch>>,
    /// The tool that lists tool templates; `only`, the templates the
    /// filter passes, and its own list requests narrow within them. See
    /// [`Filter`](crate::daemon::endpoints::tools::templates::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_templates_list: Option<Change<Reach<tools::templates::list::client::request::Filter>>>,
    /// The tool that gets one tool template by id; `only`, the
    /// templates the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::templates::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_templates_get: Option<Change<Reach<tools::templates::list::client::request::Filter>>>,
    /// The tool that deletes tool templates; `only`, the templates the
    /// filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::templates::list::client::request::Filter).
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_templates_delete: Option<Change<Reach<tools::templates::list::client::request::Filter>>>,
    /// The tool that tags tool templates: `disabled`, or `only` which
    /// templates and with which tags, each side `any` or `only`: see
    /// [`ToolsTemplatesTag`].
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_templates_tag: Option<Change<Held<ToolsTemplatesTag>>>,
    /// The tool that untags tool templates: `disabled`, or `only` which
    /// templates and of which tags, each side `any` or `only`: see
    /// [`ToolsTemplatesUntag`].
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_templates_untag: Option<Change<Held<ToolsTemplatesUntag>>>,
    /// The tool that transfers files — between the container's own
    /// filesystem, the tool containers attached to it, and the
    /// resources — and the edges it may transfer along: `any`, or
    /// `only` these, each a source and a destination. A transfer into
    /// the resources is an upload, and this is the one way a container
    /// uploads. See [`Edge`].
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer: Option<Change<Reach<Vec<Edge>>>>,
    /// The tool that lists resources; `only`, those named by id, the
    /// hash an upload answered, and a list answers none outside them.
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources_list: Option<Change<Reach<Vec<String>>>>,
    /// The tool that deletes resources; `only`, those named by id; a
    /// request naming one outside them is refused, and nothing changes.
    /// Absent, as it is; `delete`, `disabled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources_delete: Option<Change<Reach<Vec<String>>>>,
    /// The deployer: see the create's
    /// [`deployer_agent`](crate::daemon::create::Inner::deployer_agent).
    /// Absent, as it is; `delete`, none, and the container's
    /// dependencies are routed or unmet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployer_agent: Option<Change<reference::Agent>>,
}
