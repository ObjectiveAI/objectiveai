//! Text-to-speech.

use serde::{Deserialize, Serialize};

/// Text-to-speech, with its arguments. The field being absent from
/// [`Toolsets`](super::Toolsets) is Hermes's own default for this
/// toolset; present is the switch thrown on. Keyless by default —
/// Hermes's edge-tts path needs nothing — with one keyed upgrade.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Toolset {
    /// ElevenLabs, applied as `ELEVENLABS_API_KEY` in the
    /// gateway's process environment; when present the harness
    /// also selects the provider (`tts.provider: elevenlabs` in
    /// the config it owns). Hermes's other keyed voices are
    /// deliberately absent at this pin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elevenlabs_api_key: Option<String>,
}
