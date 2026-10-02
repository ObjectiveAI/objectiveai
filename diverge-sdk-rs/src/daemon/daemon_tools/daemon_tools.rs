//! Which of the daemon's tools an agent or a tool holds, and how far
//! each reaches.

use serde::{Deserialize, Serialize};

use super::{AgentsList, AgentsMessage, AgentsLogs, AgentsCreate, AgentsDelete, AgentsEdit, AgentsTag, AgentsUntag, AgentsTemplatesDelete, ToolsList, ToolsCreate, ToolsEdit, ToolsDelete, ToolsAttach, ToolsDetach, ToolsTag, ToolsUntag, ToolsTemplatesDelete};

/// The daemon's tools an agent, or a tool, made from the template
/// holds, one member each. A member present gives it that tool, as
/// far as the member says; a member absent withholds it. Part of the
/// template, and so of its hash; every member absent is `{}`, which
/// is not the same bytes as no `daemon_tools` at all.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DaemonTools {
    /// The tool that lists agents: see [`AgentsList`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_list: Option<AgentsList>,
    /// The tool that messages agents: see [`AgentsMessage`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_message: Option<AgentsMessage>,
    /// The tool that reads agents' logs: see [`AgentsLogs`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_logs: Option<AgentsLogs>,
    /// The tool that makes agents: see [`AgentsCreate`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_create: Option<AgentsCreate>,
    /// The tool that deletes agents: see [`AgentsDelete`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_delete: Option<AgentsDelete>,
    /// The tool that edits agents: see [`AgentsEdit`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_edit: Option<AgentsEdit>,
    /// The tool that tags agents: see [`AgentsTag`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_tag: Option<AgentsTag>,
    /// The tool that untags agents: see [`AgentsUntag`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_untag: Option<AgentsUntag>,
    /// The tool that makes agent templates: `true`, it has it;
    /// `false`, or absent, it does not.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub agents_templates_create: bool,
    /// The tool that deletes agent templates: see [`AgentsTemplatesDelete`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_templates_delete: Option<AgentsTemplatesDelete>,
    /// The tool that lists tools: see [`ToolsList`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_list: Option<ToolsList>,
    /// The tool that makes tools: see [`ToolsCreate`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_create: Option<ToolsCreate>,
    /// The tool that edits tools: see [`ToolsEdit`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_edit: Option<ToolsEdit>,
    /// The tool that deletes tools: see [`ToolsDelete`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_delete: Option<ToolsDelete>,
    /// The tool that attaches tools to agents: see [`ToolsAttach`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_attach: Option<ToolsAttach>,
    /// The tool that detaches tools from agents: see [`ToolsDetach`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_detach: Option<ToolsDetach>,
    /// The tool that tags tools: see [`ToolsTag`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_tag: Option<ToolsTag>,
    /// The tool that untags tools: see [`ToolsUntag`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_untag: Option<ToolsUntag>,
    /// The tool that makes tool templates: `true`, it has it;
    /// `false`, or absent, it does not.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub tools_templates_create: bool,
    /// The tool that deletes tool templates: see [`ToolsTemplatesDelete`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_templates_delete: Option<ToolsTemplatesDelete>,
}
