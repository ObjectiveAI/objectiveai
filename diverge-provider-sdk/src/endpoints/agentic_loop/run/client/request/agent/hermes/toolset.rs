//! Hermes's local capabilities, by toolset.

use serde::{Deserialize, Serialize};

/// One of Hermes's configurable toolsets — the 26-name checklist
/// its own configuration wizard manages, closed at the pin.
///
/// These are the agent's LOCAL capabilities: what Hermes itself can
/// do beside the conversation. The caller's MCP tools are not in
/// this vocabulary and never need to be — each configured MCP
/// server becomes its own toolset in Hermes automatically, and the
/// container always wires the caller's in.
///
/// Some members reach for credentials or hardware the container may
/// not have (spotify, homeassistant, computer_use, x_search); naming
/// one without its prerequisites is not a protocol error — the tools
/// simply fail as themselves when used.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Toolset {
    /// Web search and page extraction.
    Web,
    /// Browser automation.
    Browser,
    /// Terminal and process control.
    Terminal,
    /// File operations: read, write, patch, search.
    File,
    /// Code execution.
    CodeExecution,
    /// Image analysis.
    Vision,
    /// Video analysis.
    Video,
    /// Image generation.
    ImageGen,
    /// Video generation.
    VideoGen,
    /// X (Twitter) search.
    XSearch,
    /// Text-to-speech.
    Tts,
    /// Speech-to-text.
    Stt,
    /// The skills tools: list, view, manage.
    Skills,
    /// Task planning (todo).
    Todo,
    /// Persistent memory across sessions.
    Memory,
    /// The active context engine's runtime tools.
    ContextEngine,
    /// Searching past conversations.
    SessionSearch,
    /// Clarifying questions.
    Clarify,
    /// Task delegation to child agents.
    Delegation,
    /// Cron jobs.
    Cronjob,
    /// Home Assistant control.
    Homeassistant,
    /// Spotify.
    Spotify,
    /// Discord, read and participate.
    Discord,
    /// Discord server administration.
    DiscordAdmin,
    /// Yuanbao.
    Yuanbao,
    /// Desktop control via a driver.
    ComputerUse,
}
