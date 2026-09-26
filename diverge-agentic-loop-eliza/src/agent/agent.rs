//! The Eliza agent.

use diverge_sdk::shared::containers::tools::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Character, Memory, Plugin};

/// An agent running against Eliza (elizaOS).
///
/// Every field here is HONORABLE: it names something the harness
/// hands the runtime it constructs — a character field Eliza renders
/// into its prompts, a constructor option, a plugin the container
/// loads — and nothing else made the cut. Nothing here is a secret:
/// every credential a plugin needs lives in the caller's vault, under
/// a key the plugin's entry names, and is read per run. The one place
/// this vocabulary bends the protocol's post-transform rule is
/// [`character`](Self::character), and the bend is stated there.
///
/// # Nothing is pre-wired but the adapter
///
/// The harness itself loads exactly two plugins: `@elizaos/plugin-sql`,
/// the database adapter, pointed at the caller's database; and its own
/// diverge plugin, the caller's MCP tools and resources. Every other
/// plugin — the model that speaks, the one that embeds, the tools —
/// is the caller's to list here, by package name, with its own
/// settings and secrets. The packages the image carries are
/// pre-installed and nothing more: listed, they load at the image's
/// pin; unlisted, they are not touched. An agent that lists no
/// [`model_provider_plugins`](Self::model_provider_plugins) has no
/// model, and its run fails as Eliza's own failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Agent {
    /// Who the agent is: the fields Eliza re-renders into its prompt
    /// EVERY turn. A parameter of the call, not a transform applied
    /// upstream — Eliza will not take a finished prompt, so a caller
    /// who authored a personality renders it into these fields. See
    /// [`Character`].
    pub character: Character,
    /// What a turn accumulates beyond the conversation itself —
    /// reflection, long-term memory, relationships, documents,
    /// planning — each a cost per turn and a growth of the
    /// continuation. Absent is everything off. See [`Memory`].
    #[serde(default)]
    pub memory: Memory,
    /// Whether the model may GENERATE media: core's own
    /// `GENERATE_MEDIA` action, registered unconditionally and
    /// unregistered after initialize when this is off. Absent is off.
    /// The action's images come from whichever model-provider plugin
    /// registers an image-generation tier; none, and the action fails
    /// as itself. The tiers that READ media — image description,
    /// transcription — are not this switch's: a tool's image or audio
    /// is read to the model by whichever provider registers them, and
    /// described as bytes otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generate_media: Option<bool>,
    /// The plugins that register model handlers — the model that
    /// speaks, the one that embeds, the one that describes an image —
    /// in PRIORITY ORDER: the first listed answers every model type it
    /// registers, the next is its failover, and so on. See [`Plugin`],
    /// and the rule in [the agent module](super): a plugin that
    /// registers a model handler belongs here and nowhere else, and
    /// the run refuses one listed under [`plugins`](Self::plugins).
    /// Absent is no model at all.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_provider_plugins: Vec<Plugin>,
    /// Every other plugin: actions, providers, services, evaluators —
    /// any npm package that is an elizaOS plugin and registers no
    /// model handler. The run refuses one that does. Absent is none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<Plugin>,
    /// The tool containers this agent depends on, each one the caller
    /// runs and serves to it as an MCP server, in the form the
    /// provider's wire defines. Passed back whole as the registration's
    /// answer, which is how the caller learns of them. Absent is none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_tools: Vec<Tool>,
}
