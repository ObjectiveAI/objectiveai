//! Shared agent-input shape for `agents spawn` and `agents message`.
//!
//! Both commands address an agent the same three ways:
//!
//! - **Ref** — a concrete agent definition: a remote-path string
//!   (`--agent`), inline JSON (`--agent-inline`), a JSON file
//!   (`--agent-file`), or Python producing the JSON
//!   (`--agent-python-inline` / `--agent-python-file`). File/Python
//!   variants travel UNRESOLVED on the wire; the CLI resolves them
//!   into a typed agent at execution time.
//! - **Tag** — `--agent-tag <name>`, resolved against the tags DB at
//!   call time: BOUND yields the live `agent_instance_hierarchy`
//!   (historic instance), GROUPED yields the group's stored agent
//!   spec (fresh spawn that upgrades the group on first conduit
//!   read), ABSENT is an error.
//! - **Instance** — `--agent-instance <leaf>` plus optional
//!   `--parent-agent-instance-hierarchy` (the CLI substitutes its own
//!   `Config.agent_instance_hierarchy` when omitted): an existing
//!   agent instance, resumed via its stored session + continuation.

use crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.AgentSelector")]
pub enum AgentSelector {
    #[schemars(title = "Ref")]
    Ref { agent: AgentRef },
    #[schemars(title = "Tag")]
    Tag { agent_tag: String },
    #[schemars(title = "Instance")]
    Instance {
        /// Lineage prefix to prepend to `agent_instance`. When
        /// `None`, the CLI substitutes its own
        /// `Config.agent_instance_hierarchy`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        parent_agent_instance_hierarchy: Option<String>,
        /// Leaf id of the target agent instance.
        agent_instance: String,
    },
}

/// Wire form of a direct agent definition. `Resolved` carries the
/// typed inline-or-remote spec (the `--agent <ref>` string is parsed
/// into `Resolved(Remote(..))` at argv-parse time); the other
/// variants defer IO / Python execution to the CLI handler.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.AgentRef")]
pub enum AgentRef {
    #[schemars(title = "Resolved")]
    Resolved(InlineAgentBaseWithFallbacksOrRemoteCommitOptional),
    #[schemars(title = "File")]
    File(std::path::PathBuf),
    #[schemars(title = "PythonInline")]
    PythonInline(String),
    #[schemars(title = "PythonFile")]
    PythonFile(std::path::PathBuf),
}

impl AgentSelector {
    /// Append this selector's flag pair(s) to `out`. Mirrors
    /// [`super::message::RequestMessage::push_flags`] — both
    /// `agents spawn` and `agents message` accept the same flags.
    /// `Resolved` refs are emitted as `--agent-inline <json>` (the
    /// typed value round-trips identically for both Inline and
    /// Remote variants).
    pub fn push_flags(&self, out: &mut Vec<String>) {
        match self {
            AgentSelector::Ref { agent } => match agent {
                AgentRef::Resolved(spec) => {
                    out.push("--agent-inline".to_string());
                    out.push(
                        serde_json::to_string(spec)
                            .expect("agent spec serializes"),
                    );
                }
                AgentRef::File(p) => {
                    out.push("--agent-file".to_string());
                    out.push(p.to_string_lossy().into_owned());
                }
                AgentRef::PythonInline(code) => {
                    out.push("--agent-python-inline".to_string());
                    out.push(code.clone());
                }
                AgentRef::PythonFile(p) => {
                    out.push("--agent-python-file".to_string());
                    out.push(p.to_string_lossy().into_owned());
                }
            },
            AgentSelector::Tag { agent_tag } => {
                out.push("--agent-tag".to_string());
                out.push(agent_tag.clone());
            }
            AgentSelector::Instance {
                parent_agent_instance_hierarchy,
                agent_instance,
            } => {
                out.push("--agent-instance".to_string());
                out.push(agent_instance.clone());
                if let Some(parent) = parent_agent_instance_hierarchy {
                    out.push("--parent-agent-instance-hierarchy".to_string());
                    out.push(parent.clone());
                }
            }
        }
    }
}

/// Required agent-selector group: exactly one of the seven selector
/// flags must be set. `--parent-agent-instance-hierarchy` rides
/// OUTSIDE the group (it modifies `--agent-instance` rather than
/// competing with it).
#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("agent_selector")
        .required(true)
        .multiple(false)
        .args([
            "agent",
            "agent_inline",
            "agent_file",
            "agent_python_inline",
            "agent_python_file",
            "agent_tag",
            "agent_instance",
        ])
))]
pub struct AgentSelectorArgs {
    /// Remote-path string.
    #[arg(long)]
    pub agent: Option<String>,
    /// Inline JSON for the full agent definition.
    #[arg(long)]
    pub agent_inline: Option<String>,
    /// Path to a JSON file containing the agent definition.
    #[arg(long)]
    pub agent_file: Option<std::path::PathBuf>,
    /// Inline Python code that produces the agent definition.
    #[arg(long)]
    pub agent_python_inline: Option<String>,
    /// Path to a Python file that produces the agent definition.
    #[arg(long)]
    pub agent_python_file: Option<std::path::PathBuf>,
    /// Tag name, resolved against the tags DB at execution time:
    /// BOUND → the live hierarchy, GROUPED → the group's stored
    /// agent spec (the whole group flips to BOUND on the spawn's
    /// hierarchy via the conduit-driven upgrade), ABSENT → error.
    #[arg(long)]
    pub agent_tag: Option<String>,
    /// Leaf id of an existing agent instance.
    #[arg(long)]
    pub agent_instance: Option<String>,
    /// Optional lineage prefix to prepend to `--agent-instance`.
    /// When omitted, the cli substitutes its own
    /// `Config.agent_instance_hierarchy`.
    #[arg(long, requires = "agent_instance")]
    pub parent_agent_instance_hierarchy: Option<String>,
}

impl TryFrom<AgentSelectorArgs> for AgentSelector {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(args: AgentSelectorArgs) -> Result<Self, Self::Error> {
        if let Some(s) = args.agent_inline {
            let mut de = serde_json::Deserializer::from_str(&s);
            let spec: InlineAgentBaseWithFallbacksOrRemoteCommitOptional =
                serde_path_to_error::deserialize(&mut de).map_err(|source| {
                    crate::cli::command::FromArgsError {
                        field: "agent_inline",
                        source: source.into(),
                    }
                })?;
            Ok(AgentSelector::Ref {
                agent: AgentRef::Resolved(spec),
            })
        } else if let Some(s) = args.agent {
            let path: crate::RemotePathCommitOptional = s
                .parse()
                .map_err(|e| crate::cli::command::FromArgsError::path_parse("agent", e))?;
            Ok(AgentSelector::Ref {
                agent: AgentRef::Resolved(
                    InlineAgentBaseWithFallbacksOrRemoteCommitOptional::Remote(path),
                ),
            })
        } else if let Some(p) = args.agent_file {
            Ok(AgentSelector::Ref {
                agent: AgentRef::File(p),
            })
        } else if let Some(code) = args.agent_python_inline {
            Ok(AgentSelector::Ref {
                agent: AgentRef::PythonInline(code),
            })
        } else if let Some(p) = args.agent_python_file {
            Ok(AgentSelector::Ref {
                agent: AgentRef::PythonFile(p),
            })
        } else if let Some(agent_tag) = args.agent_tag {
            Ok(AgentSelector::Tag { agent_tag })
        } else {
            // Clap `required = true` on the `agent_selector` group
            // guarantees exactly one of the seven flags is set.
            Ok(AgentSelector::Instance {
                parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
                agent_instance: args.agent_instance.unwrap(),
            })
        }
    }
}
