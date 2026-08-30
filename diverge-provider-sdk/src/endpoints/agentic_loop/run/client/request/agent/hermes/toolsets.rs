//! Hermes's local capabilities, one switch each.

use serde::{Deserialize, Serialize};

/// Per-toolset switches over Hermes's 26-name configurable
/// checklist — the list its own configuration wizard manages,
/// closed at the pin.
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
/// Some members reach for credentials or hardware the container may
/// not have (spotify, homeassistant, computer_use, x_search);
/// enabling one without its prerequisites is not a protocol error —
/// the tools simply fail as themselves when used.
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
    /// Speech-to-text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stt: Option<bool>,
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
    /// Clarifying questions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clarify: Option<bool>,
    /// Task delegation to child agents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegation: Option<bool>,
    /// Cron jobs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cronjob: Option<bool>,
    /// Home Assistant control.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homeassistant: Option<bool>,
    /// Spotify.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spotify: Option<bool>,
    /// Discord, read and participate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discord: Option<bool>,
    /// Discord server administration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discord_admin: Option<bool>,
    /// Yuanbao.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yuanbao: Option<bool>,
    /// Desktop control via a driver.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub computer_use: Option<bool>,
}

impl Toolsets {
    /// Whether nothing here says anything — every switch at its
    /// absent tri-state. What lets the whole struct stay off the
    /// wire when the caller changed nothing.
    pub fn unsaid(&self) -> bool {
        self == &Self::default()
    }
}
