//! The toolsets — every switch in one place.

use serde::{Deserialize, Serialize};

/// Per-toolset switches over Hermes's configurable checklist — the
/// list its own configuration wizard manages, closed at the pin,
/// and narrowed to what a REQUEST can actually make work.
///
/// Every field is one toolset's own `Toolset` union: absent is
/// Hermes's own default for that toolset, `false` turns it off,
/// `true` turns it on with nothing supplied, and an object turns it
/// on WITH ARGUMENTS — that toolset's own credentials and endpoint
/// facts, each documented in its own file with the mechanism the
/// harness applies. Enabling a tool without its prerequisites is
/// not a protocol error — the tools simply fail as themselves when
/// used.
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
/// What is deliberately ABSENT, and why, is [the
/// module](super)'s to say.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Toolsets {
    /// Web search and page extraction. See [`web`](super::web).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web: Option<super::web::Toolset>,
    /// Browser automation. See [`browser`](super::browser).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<super::browser::Toolset>,
    /// Terminal and process control. See
    /// [`terminal`](super::terminal).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<super::terminal::Toolset>,
    /// File operations. See [`file`](super::file).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<super::file::Toolset>,
    /// Code execution. See
    /// [`code_execution`](super::code_execution).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_execution: Option<super::code_execution::Toolset>,
    /// Image analysis. See [`vision`](super::vision).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vision: Option<super::vision::Toolset>,
    /// Video analysis. See [`video`](super::video).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<super::video::Toolset>,
    /// Image generation. See [`image_gen`](super::image_gen).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_gen: Option<super::image_gen::Toolset>,
    /// Video generation. See [`video_gen`](super::video_gen).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_gen: Option<super::video_gen::Toolset>,
    /// X (Twitter) search. See [`x_search`](super::x_search).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_search: Option<super::x_search::Toolset>,
    /// Text-to-speech. See [`tts`](super::tts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tts: Option<super::tts::Toolset>,
    /// Task planning. See [`todo`](super::todo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub todo: Option<super::todo::Toolset>,
    /// Searching past conversations. See
    /// [`session_search`](super::session_search).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_search: Option<super::session_search::Toolset>,
    /// Task delegation to child agents. See
    /// [`delegation`](super::delegation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegation: Option<super::delegation::Toolset>,
    /// Home Assistant control. See
    /// [`homeassistant`](super::homeassistant).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homeassistant: Option<super::homeassistant::Toolset>,
    /// Spotify control. See [`spotify`](super::spotify).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spotify: Option<super::spotify::Toolset>,
}

impl Toolsets {
    /// Whether nothing here says anything — every switch at its
    /// absent tri-state. What lets the whole struct stay off the
    /// wire when the caller changed nothing.
    pub fn unsaid(&self) -> bool {
        self == &Self::default()
    }
}
