//! The members an agent's create and a tool's create share.

use serde::{Deserialize, Serialize};

use crate::daemon::daemon_tools::{
    AgentsTag, AgentsTemplatesTag, AgentsTemplatesUntag, AgentsUntag, Edge, Held, Reach, Switch, ToolsAttach,
    ToolsDetach, ToolsTag, ToolsTemplatesTag, ToolsTemplatesUntag, ToolsUntag,
};
use crate::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use crate::daemon::endpoints::{agents, tools};
use crate::daemon::reference;

/// Everything a container is made of that its template does not say
/// and its name is not: the template, the provider pin and its
/// volumes, the FUSE mounts, the daemon's own tools one member each,
/// the deployer. Flattened into an
/// [agent's](crate::daemon::endpoints::agents::create) and a
/// [tool's](crate::daemon::endpoints::tools::create) create, so its
/// members are the request's own.
///
/// # The daemon's tools
///
/// One member for every endpoint but
/// [`tools::connect`](crate::daemon::endpoints::tools::connect),
/// which joins somebody else's container on an authorization of the
/// caller's and is the caller's alone. Every one is a [`Reach`] —
/// `"disabled"`, `"any"`, or what it names, flat — or, for a tool
/// with nothing to narrow, a [`Switch`], or, for one that tags, a
/// [`Held`], `"disabled"` or its two sides, each `"any"` or what it
/// names. Every one is always present, `disabled` when the container
/// does not hold the tool, so that an edit replaces a member and
/// never adds or removes one. What `only` names is a filter, the very
/// shape the list of that family narrows by, read as a test — see
/// [`daemon_tools`](crate::daemon::daemon_tools) — or a pair of
/// them, or a filter with its tags, or a list of ids. Which tools a
/// container holds is its for its life; how far each reaches is
/// edited.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Inner {
    /// The template the container is made from, by its id — the hash
    /// its family's `templates::create` answered. An id no template
    /// of the caller's has, or one of the other family, is the
    /// create's error. The template's image, limits, resources and
    /// arguments are the container's for its life.
    pub template: String,
    /// The one provider the container runs on, and the volumes of that
    /// provider made visible inside it. See [`Provider`].
    ///
    /// Absent, the container runs on whichever provider the daemon
    /// chooses, and mounts no volume: a volume is a provider's own
    /// and does not carry across, so a container with state on a
    /// provider's disk is a container of that provider. The
    /// container's for its life, and never the template's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    /// Files of providers' volumes served LIVE across the daemon,
    /// mounted one each over FUSE.
    ///
    /// Each names a file by a provider, a volume of that provider's
    /// and a path in it, and its path in the container — see
    /// [`FuseMount`]; each may name a different provider. Every one
    /// is mounted before the container runs, and every read and every
    /// write of a piece inside the container is one ask, forwarded by
    /// the daemon to the volume's provider and served from the
    /// volume's file in place. The file is
    /// overwritten in place only; a program that replaces its file by
    /// rename needs a directory mount. Its container path is no other
    /// mount's — the template's resource mounts included — and lies
    /// inside none, as every mount's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of providers' volumes served LIVE across the
    /// daemon, mounted one each over FUSE.
    ///
    /// Each names a directory by a provider, a volume of that
    /// provider's and a path in it, and its path in the container —
    /// see [`FuseMount`]; each may name a different provider. The
    /// whole tree under the volume path is what the container sees:
    /// every listing, stat, read or write of a piece, truncation,
    /// change of attributes, creation, removal and rename inside the
    /// container is one ask, forwarded by the daemon to the volume's
    /// provider and served from the volume's tree in place. No other
    /// mount may lie inside it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_directory_mounts: Vec<FuseMount>,
    /// The tool that lists agents; `only`, the agents the filter
    /// passes, and its own list requests narrow within them. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    #[serde(default)]
    pub agents_list: Reach<agents::list::client::request::Filter>,
    /// The tool that gets one agent as a list would report it; `only`,
    /// the agents the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    #[serde(default)]
    pub agents_get: Reach<agents::list::client::request::Filter>,
    /// The tool that messages agents; `only`, the agents the filter
    /// passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    #[serde(default)]
    pub agents_message: Reach<agents::list::client::request::Filter>,
    /// The tool that reads agents' logs; `only`, the agents the filter
    /// passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    #[serde(default)]
    pub agents_logs: Reach<agents::list::client::request::Filter>,
    /// The tool that makes agents; `only`, from the agent templates the
    /// filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    #[serde(default)]
    pub agents_create: Reach<agents::templates::list::client::request::Filter>,
    /// The tool that deletes agents; `only`, the agents the filter
    /// passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    #[serde(default)]
    pub agents_delete: Reach<agents::list::client::request::Filter>,
    /// The tool that edits agents; `only`, the agents the filter
    /// passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    #[serde(default)]
    pub agents_edit: Reach<agents::list::client::request::Filter>,
    /// The tool that tags agents: `disabled`, or `only` which agents
    /// and with which tags, each side `any` or `only`: see
    /// [`AgentsTag`].
    #[serde(default)]
    pub agents_tag: Held<AgentsTag>,
    /// The tool that untags agents: `disabled`, or `only` which agents
    /// and of which tags, each side `any` or `only`: see
    /// [`AgentsUntag`].
    #[serde(default)]
    pub agents_untag: Held<AgentsUntag>,
    /// The tool that makes agent templates: see [`Switch`].
    #[serde(default)]
    pub agents_templates_create: Switch,
    /// The tool that lists agent templates; `only`, the templates the
    /// filter passes, and its own list requests narrow within them. See
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    #[serde(default)]
    pub agents_templates_list: Reach<agents::templates::list::client::request::Filter>,
    /// The tool that gets one agent template by id; `only`, the
    /// templates the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    #[serde(default)]
    pub agents_templates_get: Reach<agents::templates::list::client::request::Filter>,
    /// The tool that deletes agent templates; `only`, the templates the
    /// filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    #[serde(default)]
    pub agents_templates_delete: Reach<agents::templates::list::client::request::Filter>,
    /// The tool that tags agent templates: `disabled`, or `only` which
    /// templates and with which tags, each side `any` or `only`: see
    /// [`AgentsTemplatesTag`].
    #[serde(default)]
    pub agents_templates_tag: Held<AgentsTemplatesTag>,
    /// The tool that untags agent templates: `disabled`, or `only`
    /// which templates and of which tags, each side `any` or `only`:
    /// see [`AgentsTemplatesUntag`].
    #[serde(default)]
    pub agents_templates_untag: Held<AgentsTemplatesUntag>,
    /// The tool that lists tools; `only`, the tools the filter passes,
    /// and its own list requests narrow within them. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    #[serde(default)]
    pub tools_list: Reach<tools::list::client::request::Filter>,
    /// The tool that gets one tool as a list would report it; `only`,
    /// the tools the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    #[serde(default)]
    pub tools_get: Reach<tools::list::client::request::Filter>,
    /// The tool that makes tools; `only`, from the tool templates the
    /// filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::templates::list::client::request::Filter).
    #[serde(default)]
    pub tools_create: Reach<tools::templates::list::client::request::Filter>,
    /// The tool that edits tools; `only`, the tools the filter passes.
    /// See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    #[serde(default)]
    pub tools_edit: Reach<tools::list::client::request::Filter>,
    /// The tool that deletes tools; `only`, the tools the filter
    /// passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    #[serde(default)]
    pub tools_delete: Reach<tools::list::client::request::Filter>,
    /// The tool that attaches tools to agents; `only`, what it may join
    /// to what: see [`ToolsAttach`].
    #[serde(default)]
    pub tools_attach: Reach<ToolsAttach>,
    /// The tool that detaches tools from agents; `only`, what it may
    /// part from what: see [`ToolsDetach`].
    #[serde(default)]
    pub tools_detach: Reach<ToolsDetach>,
    /// The tool that tags tools: `disabled`, or `only` which tools and
    /// with which tags, each side `any` or `only`: see [`ToolsTag`].
    #[serde(default)]
    pub tools_tag: Held<ToolsTag>,
    /// The tool that untags tools: `disabled`, or `only` which tools
    /// and of which tags, each side `any` or `only`: see
    /// [`ToolsUntag`].
    #[serde(default)]
    pub tools_untag: Held<ToolsUntag>,
    /// The tool that routes a dependency position to a tool; `only`,
    /// to the tools the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    #[serde(default)]
    pub tools_routes_add: Reach<tools::list::client::request::Filter>,
    /// The tool that takes a route up; `only`, the routes the filter
    /// passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::routes::list::client::request::Filter).
    #[serde(default)]
    pub tools_routes_delete: Reach<tools::routes::list::client::request::Filter>,
    /// The tool that lists routes; `only`, the routes the filter
    /// passes, and its own list requests narrow within them. See
    /// [`Filter`](crate::daemon::endpoints::tools::routes::list::client::request::Filter).
    #[serde(default)]
    pub tools_routes_list: Reach<tools::routes::list::client::request::Filter>,
    /// The tool that makes tool templates: see [`Switch`].
    #[serde(default)]
    pub tools_templates_create: Switch,
    /// The tool that lists tool templates; `only`, the templates the
    /// filter passes, and its own list requests narrow within them. See
    /// [`Filter`](crate::daemon::endpoints::tools::templates::list::client::request::Filter).
    #[serde(default)]
    pub tools_templates_list: Reach<tools::templates::list::client::request::Filter>,
    /// The tool that gets one tool template by id; `only`, the
    /// templates the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::templates::list::client::request::Filter).
    #[serde(default)]
    pub tools_templates_get: Reach<tools::templates::list::client::request::Filter>,
    /// The tool that deletes tool templates; `only`, the templates the
    /// filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::templates::list::client::request::Filter).
    #[serde(default)]
    pub tools_templates_delete: Reach<tools::templates::list::client::request::Filter>,
    /// The tool that tags tool templates: `disabled`, or `only` which
    /// templates and with which tags, each side `any` or `only`: see
    /// [`ToolsTemplatesTag`].
    #[serde(default)]
    pub tools_templates_tag: Held<ToolsTemplatesTag>,
    /// The tool that untags tool templates: `disabled`, or `only` which
    /// templates and of which tags, each side `any` or `only`: see
    /// [`ToolsTemplatesUntag`].
    #[serde(default)]
    pub tools_templates_untag: Held<ToolsTemplatesUntag>,
    /// The tool that transfers files — between the container's own
    /// filesystem, the tool containers attached to it, and the
    /// resources — and the edges it may transfer along: `any`, or
    /// `only` these, each a source and a destination. A transfer into
    /// the resources is an upload, and this is the one way a container
    /// uploads. See [`Edge`].
    #[serde(default)]
    pub transfer: Reach<Vec<Edge>>,
    /// The tool that lists resources; `only`, those named by id, the
    /// hash an upload answered, and a list answers none outside them.
    #[serde(default)]
    pub resources_list: Reach<Vec<String>>,
    /// The tool that deletes resources; `only`, those named by id; a
    /// request naming one outside them is refused, and nothing changes.
    #[serde(default)]
    pub resources_delete: Reach<Vec<String>>,
    /// The agent of the caller's the daemon hands this container's
    /// declared tool dependencies to — each as the template and the
    /// instructions the container returned at register time — when no
    /// [route](crate::daemon::endpoints::tools::routes) answers them.
    /// The deployer makes the tool, attaches it, and may add a route so
    /// that the next ask at that position is answered without it. By
    /// name, or by template and index: see [`reference::Agent`].
    /// Absent, the daemon deploys nothing itself: a dependency no
    /// route answers is not met, and the container's tools channel is
    /// answered with an error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployer_agent: Option<reference::Agent>,
}
