//! Hermes's local capabilities, one switch each.

use serde::{Deserialize, Serialize};

/// Per-toolset switches over Hermes's configurable checklist — the
/// list its own configuration wizard manages, closed at the pin,
/// and narrowed to what a REQUEST can actually make work: every
/// switch here names a capability the request's own provisioning
/// can enable — tool credentials in the environment, skills and
/// data as mounts.
///
/// Every switch is a tri-state: absent means Hermes's own default
/// for that toolset, `true` turns it on, `false` turns it off — so
/// a caller says only what it means to change, and the unsaid rest
/// tracks Hermes's defaults instead of freezing a copy of them.
///
/// These are the agent's LOCAL capabilities: what Hermes itself can
/// do beside the conversation. The caller's MCP tools are not in
/// this vocabulary and never need to be — each configured MCP
/// server becomes its own toolset in Hermes automatically, and the
/// container always wires the caller's in. The SKILLS toolset is
/// not here either, on purpose: whether the agent can browse skills
/// follows from whether the request mounted any, not from a switch
/// that could contradict that fact.
///
/// Deliberately ABSENT, because no mount can make them work here:
/// `computer_use` (a running display server, not a file), `stt`
/// (not a model toolset at all), `clarify` (needs wiring Hermes's
/// wire does not have), `discord`/`discord_admin` (Hermes
/// hard-restricts them to its Discord platform, which is not the
/// surface a container run speaks); `cronjob`, whose whole purpose
/// is execution at a future time, a thing an ephemeral one-run
/// container does not have; and `spotify`, whose ONLY auth is
/// rotating OAuth — the container's first token refresh would
/// consume the caller's single-use rotation and burn their local
/// login, a harm no provisioning can fix.
///
/// Some members reach for credentials or services the container may
/// not have (homeassistant, x_search, yuanbao); enabling one
/// without its prerequisites is not a protocol error — the tools
/// simply fail as themselves when used. Where a member has both a
/// key path and an OAuth path (x_search), only the key path is
/// container-safe: rotating OAuth state is not an argument.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
)]
pub struct Toolsets {
    /// Web search and page extraction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web: Option<bool>,
    /// Browser automation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<bool>,
    /// Terminal and process control.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<bool>,
    /// File operations: read, write, patch, search.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<bool>,
    /// Code execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_execution: Option<bool>,
    /// Image analysis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vision: Option<bool>,
    /// Video analysis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<bool>,
    /// Image generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_gen: Option<bool>,
    /// Video generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_gen: Option<bool>,
    /// X (Twitter) search.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_search: Option<bool>,
    /// Text-to-speech.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,
    /// Task planning (todo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub todo: Option<bool>,
    /// Persistent memory across sessions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<bool>,
    /// The active context engine's runtime tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_engine: Option<bool>,
    /// Searching past conversations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_search: Option<bool>,
    /// Task delegation to child agents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegation: Option<bool>,
    /// Home Assistant control.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homeassistant: Option<bool>,
    /// Yuanbao.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yuanbao: Option<bool>,
}

impl Toolsets {
    /// Whether nothing here says anything — every switch at its
    /// absent tri-state. What lets the whole struct stay off the
    /// wire when the caller changed nothing.
    pub fn unsaid(&self) -> bool {
        self == &Self::default()
    }
}
