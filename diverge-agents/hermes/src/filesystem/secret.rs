//! A secret the run takes, and where its value goes.

use diverge_sdk::shared::containers::secrets::Mode;

/// One secret the agent's arguments select: the name the template
/// declares it under — the proxy feeds a value for it before every
/// loop — and where the harness puts that value before
/// `hermes gateway` starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Secret {
    /// The name, as the template declares it and the feed carries
    /// it.
    pub name: &'static str,
    /// Where the value lands.
    pub target: Target,
}

/// Where a secret's value is put — which also fixes the mode the run
/// declares it in. The first three are read and never changed by
/// Hermes, so they are `fixed`; the last two are documents Hermes
/// rewrites on a token refresh, so they are `rotating`: read back
/// after the gateway exits and handed to the proxy's collect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The gateway's process environment, under the secret's own
    /// name. The value must be a JSON string.
    Env,
    /// `model.api_key` in `config.yaml` — the `custom` provider's key,
    /// which Hermes reads there and nowhere else. A JSON string.
    ModelApiKey,
    /// The service-account file `VERTEX_CREDENTIALS_PATH` names. A
    /// JSON string holding the document, written verbatim.
    VertexFile,
    /// The `providers.<entry>` object of `auth.json`. A JSON object,
    /// written as the entry and read back from it.
    AuthEntry(&'static str),
    /// The Qwen CLI's own token file at the real home. A JSON object,
    /// written as the file and read back from it.
    QwenCreds,
}

impl Target {
    /// The mode the target implies: the environment, the config key
    /// and the vertex file are read once — `fixed`; the `auth.json`
    /// entry and the Qwen file are rewritten by Hermes — `rotating`.
    pub fn mode(self) -> Mode {
        match self {
            Target::Env | Target::ModelApiKey | Target::VertexFile => Mode::Fixed,
            Target::AuthEntry(_) | Target::QwenCreds => Mode::Rotating,
        }
    }
}
