//! The toolsets — every switch in one place.

use serde::{Deserialize, Serialize};

/// Per-toolset switches over Hermes's configurable checklist — the
/// list its own configuration wizard manages, closed at the pin,
/// and narrowed to what a REQUEST can actually make work.
///
/// Two field shapes, by whether the toolset takes arguments. A
/// toolset with nothing to configure is a tri-state `Option<bool>`:
/// absent tracks Hermes's own default, `true` turns it on, `false`
/// off. A toolset with arguments is an `Option` of its own
/// structure — absent is Hermes's default, present is the switch
/// thrown on WITH that tool's credentials and endpoint facts, each
/// documented in its own file with the mechanism the harness
/// applies. Enabling a tool without its prerequisites is not a
/// protocol error — the tools simply fail as themselves when used.
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
/// What is deliberately ABSENT, and why, is [the module](super)'s
/// to say.
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
    /// Terminal and process control. Nothing to configure — the
    /// container is the sandbox, and the local backend reads no
    /// credentials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<bool>,
    /// File operations: read, write, patch, search. Nothing to
    /// configure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<bool>,
    /// Code execution. Nothing of its own to configure — scripts
    /// call other tools over RPC, and each called tool's own
    /// arguments apply transitively.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_execution: Option<bool>,
    /// Image analysis. Nothing to configure — analysis rides the
    /// run's own inference provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vision: Option<bool>,
    /// Video analysis. Nothing to configure — it rides the run's
    /// own inference provider, exactly as vision does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<bool>,
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
    /// Task planning (todo). Nothing to configure — the store is
    /// in-memory and the run's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub todo: Option<bool>,
    /// Searching past conversations. Nothing to configure — the
    /// local session database, credential-free; in a one-run
    /// container the searchable past is this run's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_search: Option<bool>,
    /// Task delegation to child agents. Nothing to configure —
    /// children inherit the run's provider credentials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegation: Option<bool>,
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
