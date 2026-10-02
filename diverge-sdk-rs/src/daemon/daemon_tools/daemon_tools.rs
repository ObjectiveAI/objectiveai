//! Which of the daemon's tools an agent or a tool holds, and how far
//! each reaches.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::{agents, tools};
use super::{AgentsTag, AgentsUntag, ToolsAttach, ToolsDetach, ToolsTag, ToolsUntag};

/// The daemon's tools an agent, or a tool, made from the template
/// holds, one member each. A member present gives it that tool, as
/// far as the member says: a filter, the very shape the list of that
/// family narrows by, read as a test — see
/// [`daemon_tools`](super) — and `{}` reaches every one. A member
/// absent withholds the tool. Part of the template, and so of its
/// hash; every member absent is `{}`, which is not the same bytes as
/// no `daemon_tools` at all.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DaemonTools {
    /// The tool that lists agents, and the agents it reaches: those the
    /// filter passes. Its own list requests narrow within these. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_list: Option<agents::list::client::request::Filter>,
    /// The tool that messages agents, and the agents it may message:
    /// those the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_message: Option<agents::list::client::request::Filter>,
    /// The tool that reads agents' logs, and the agents whose logs it
    /// may read: those the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_logs: Option<agents::list::client::request::Filter>,
    /// The tool that makes agents, and the agent templates it may make
    /// one from: those the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_create: Option<agents::templates::list::client::request::Filter>,
    /// The tool that deletes agents, and the agents it may delete:
    /// those the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_delete: Option<agents::list::client::request::Filter>,
    /// The tool that edits agents, and the agents it may edit: those
    /// the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::list::client::request::Filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_edit: Option<agents::list::client::request::Filter>,
    /// The tool that tags agents, which agents and with which tags: see
    /// [`AgentsTag`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_tag: Option<AgentsTag>,
    /// The tool that untags agents, which agents and of which tags: see
    /// [`AgentsUntag`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_untag: Option<AgentsUntag>,
    /// The tool that makes agent templates: `true`, it has it; `false`,
    /// or absent, it does not.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub agents_templates_create: bool,
    /// The tool that deletes agent templates, and the templates it may
    /// delete: those the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::agents::templates::list::client::request::Filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_templates_delete: Option<agents::templates::list::client::request::Filter>,
    /// The tool that lists tools, and the tools it reaches: those the
    /// filter passes. Its own list requests narrow within these. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_list: Option<tools::list::client::request::Filter>,
    /// The tool that makes tools, and the tool templates it may make
    /// one from: those the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::templates::list::client::request::Filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_create: Option<tools::templates::list::client::request::Filter>,
    /// The tool that edits tools, and the tools it may edit: those the
    /// filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_edit: Option<tools::list::client::request::Filter>,
    /// The tool that deletes tools, and the tools it may delete: those
    /// the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::list::client::request::Filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_delete: Option<tools::list::client::request::Filter>,
    /// The tool that attaches tools to agents, and what it may join to
    /// what: see [`ToolsAttach`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_attach: Option<ToolsAttach>,
    /// The tool that detaches tools from agents, and what it may part
    /// from what: see [`ToolsDetach`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_detach: Option<ToolsDetach>,
    /// The tool that tags tools, which tools and with which tags: see
    /// [`ToolsTag`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_tag: Option<ToolsTag>,
    /// The tool that untags tools, which tools and of which tags: see
    /// [`ToolsUntag`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_untag: Option<ToolsUntag>,
    /// The tool that makes tool templates: `true`, it has it; `false`,
    /// or absent, it does not.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub tools_templates_create: bool,
    /// The tool that deletes tool templates, and the templates it may
    /// delete: those the filter passes. See
    /// [`Filter`](crate::daemon::endpoints::tools::templates::list::client::request::Filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_templates_delete: Option<tools::templates::list::client::request::Filter>,
}
