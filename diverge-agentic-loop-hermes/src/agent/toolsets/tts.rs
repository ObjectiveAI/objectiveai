//! Text-to-speech.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Text-to-speech, with its arguments. The field being absent from
/// [`Toolsets`](super::Toolsets) is Hermes's own default for this
/// toolset; present is the switch thrown on.
///
/// The ElevenLabs switch is OPTIONAL by the keyless-floor rule:
/// Hermes's default voice is edge-tts, which needs nothing — the
/// empty object is speech on the free voice, coherently. The key is
/// the upgrade, not the existence.
///
/// SECRET: `ELEVENLABS_API_KEY`, fixed, when
/// [`elevenlabs`](Self::elevenlabs) is `true`; none otherwise.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct Toolset {
    /// ElevenLabs as the voice: `true` declares `ELEVENLABS_API_KEY`,
    /// applied in the gateway's process environment, and the harness
    /// selects the provider (`tts.provider: elevenlabs` in the config
    /// it owns). Absent or `false` is the free voice. Hermes's other
    /// keyed voices are deliberately absent at this pin.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub elevenlabs: bool,
}
