//! Which of the daemon's tools an agent or a tool holds, and how far
//! each reaches.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::{agents, tools};
use super::{AgentsTag, AgentsTemplatesTag, AgentsTemplatesUntag, AgentsUntag, ToolsAttach, ToolsDetach, ToolsTag, ToolsTemplatesTag, ToolsTemplatesUntag, ToolsUntag, Reach, Switch};

/// The daemon's tools an agent, or a tool, holds, named on its
/// create, one member each: one for every endpoint but
/// [`tools::connect`](crate::daemon::endpoints::tools::connect),
/// which joins somebody else's container on an authorization of the
/// caller's and is the caller's alone. Every member is a [`Reach`] —
/// `disabled`, `any`, or `only` what it names — or, for a tool with
/// nothing to narrow, a [`Switch`]; every member is always present,
/// `disabled` when the container does not hold the tool, so that an
/// edit replaces a member and never adds or removes one. What `only`
/// names is a filter, the very shape the list of that family narrows
/// by, read as a test — see [`daemon_tools`](super) — or a pair of
/// them, or a filter with its tags, or a list of ids. The agent's,
/// or the tool's, for its life as to which tools it holds; how far
/// each reaches is edited.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DaemonTools {
    /// The tool that lists agents; `only`, the agents the filter
    /// passes, and its own list requests narrow within them. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    #[serde(default)]
    pub agents_list: Reach<agents::list::client::request::Filter>,
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
    /// The tool that tags agents; `only`, which agents and with which
    /// tags: see [`AgentsTag`].
    #[serde(default)]
    pub agents_tag: Reach<AgentsTag>,
    /// The tool that untags agents; `only`, which agents and of which
    /// tags: see [`AgentsUntag`].
    #[serde(default)]
    pub agents_untag: Reach<AgentsUntag>,
    /// The tool that makes agent templates: see [`Switch`].
    #[serde(default)]
    pub agents_templates_create: Switch,
    /// The tool that lists agent templates; `only`, the templates the
    /// filter passes, and its own list requests narrow within them. See
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    #[serde(default)]
    pub agents_templates_list: Reach<agents::templates::list::client::request::Filter>,
    /// The tool that deletes agent templates; `only`, the templates the
    /// filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    #[serde(default)]
    pub agents_templates_delete: Reach<agents::templates::list::client::request::Filter>,
    /// The tool that tags agent templates; `only`, which templates and
    /// with which tags: see [`AgentsTemplatesTag`].
    #[serde(default)]
    pub agents_templates_tag: Reach<AgentsTemplatesTag>,
    /// The tool that untags agent templates; `only`, which templates
    /// and of which tags: see [`AgentsTemplatesUntag`].
    #[serde(default)]
    pub agents_templates_untag: Reach<AgentsTemplatesUntag>,
    /// The tool that lists tools; `only`, the tools the filter passes,
    /// and its own list requests narrow within them. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    #[serde(default)]
    pub tools_list: Reach<tools::list::client::request::Filter>,
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
    /// The tool that tags tools; `only`, which tools and with which
    /// tags: see [`ToolsTag`].
    #[serde(default)]
    pub tools_tag: Reach<ToolsTag>,
    /// The tool that untags tools; `only`, which tools and of which
    /// tags: see [`ToolsUntag`].
    #[serde(default)]
    pub tools_untag: Reach<ToolsUntag>,
    /// The tool that makes tool templates: see [`Switch`].
    #[serde(default)]
    pub tools_templates_create: Switch,
    /// The tool that lists tool templates; `only`, the templates the
    /// filter passes, and its own list requests narrow within them. See
    /// [`Filter`](crate::daemon::endpoints::tools::templates::list::client::request::Filter).
    #[serde(default)]
    pub tools_templates_list: Reach<tools::templates::list::client::request::Filter>,
    /// The tool that deletes tool templates; `only`, the templates the
    /// filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::templates::list::client::request::Filter).
    #[serde(default)]
    pub tools_templates_delete: Reach<tools::templates::list::client::request::Filter>,
    /// The tool that tags tool templates; `only`, which templates and
    /// with which tags: see [`ToolsTemplatesTag`].
    #[serde(default)]
    pub tools_templates_tag: Reach<ToolsTemplatesTag>,
    /// The tool that untags tool templates; `only`, which templates and
    /// of which tags: see [`ToolsTemplatesUntag`].
    #[serde(default)]
    pub tools_templates_untag: Reach<ToolsTemplatesUntag>,
    /// The tool that uploads resources: see [`Switch`].
    #[serde(default)]
    pub resources_upload: Switch,
    /// The tool that lists resources; `only`, those named by id, the
    /// hash an upload answered, and a list answers none outside them.
    #[serde(default)]
    pub resources_list: Reach<Vec<String>>,
    /// The tool that deletes resources; `only`, those named by id; a
    /// request naming one outside them is refused, and nothing changes.
    #[serde(default)]
    pub resources_delete: Reach<Vec<String>>,
}

impl DaemonTools {
    /// Whether every tool is `disabled`: what a container made with
    /// no `daemon_tools` holds, and what a create leaves off the
    /// wire.
    pub fn all_disabled(&self) -> bool {
        *self == Self::default()
    }
}
