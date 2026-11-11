//! What the agent adds up to, before anything is read or written.

use std::collections::BTreeMap;

use diverge_sdk::shared::containers::secrets::Declaration;

use super::{PrepareError, Secret};

/// The accumulator the provider and the toolsets write into: the
/// environment, the config's inputs, and the secrets the run takes
/// — each by name, with where its fed value goes.
#[derive(Debug, Default)]
pub struct Plan {
    /// The gateway's environment: the harness's plain variables —
    /// switches, endpoints, geometry. The secrets whose target is the
    /// environment join these at [`prepare`](super::prepare), from
    /// the fed values.
    pub env: BTreeMap<String, String>,
    /// `model.provider` — the provider's own id, as its marker
    /// serializes.
    pub provider: String,
    /// `model.default` — the model, in the provider's naming.
    pub model: String,
    /// `model.base_url` — the `custom` provider's endpoint. Hermes
    /// consults `model.api_key` ONLY when the config also carries the
    /// base URL (and `model.provider` is `custom`), so the URL goes
    /// here as well as into `CUSTOM_BASE_URL`.
    pub base_url: Option<String>,
    /// `web.search_backend`, when a search slot names one.
    pub search_backend: Option<&'static str>,
    /// `web.extract_backend`, when the extract slot names one.
    pub extract_backend: Option<&'static str>,
    /// `browser.cloud_provider: browserbase`, when the browser's remote
    /// is Browserbase — pinned, so Hermes's auto-detect walk (Browser
    /// Use first) never enters into it.
    pub browserbase: bool,
    /// `tts.provider: elevenlabs`, when the toolset asks for it.
    pub tts_elevenlabs: bool,
    /// `image_gen.provider: fal`, when the toolset is on.
    pub image_gen: bool,
    /// `video_gen.provider: fal`, when the toolset is on.
    pub video_gen: bool,
    /// `platform_toolsets.api_server`: the toolsets exposed, by
    /// Hermes's own configurable-key names, in its order.
    pub toolsets: Vec<&'static str>,
    /// The secrets the run takes, in the order the agent names them
    /// — the order the declaration lists them in.
    pub secrets: Vec<Secret>,
}

impl Plan {
    /// An empty plan for one model.
    pub fn new(model: String) -> Self {
        Plan {
            model,
            ..Plan::default()
        }
    }

    /// Set one plain environment variable. Setting it again to the
    /// SAME value is fine; to a different one is the caller's
    /// contradiction ([`PrepareError::Contradiction`]). No credential
    /// comes through here: a credential is [`declare`](Self::declare)d
    /// by name, and its value arrives with the feed.
    pub fn set(
        &mut self,
        variable: &'static str,
        value: String,
    ) -> Result<(), PrepareError> {
        match self.env.get(variable) {
            Some(existing) if *existing != value => {
                Err(PrepareError::Contradiction { variable })
            }
            Some(_) => Ok(()),
            None => {
                self.env.insert(variable.to_string(), value);
                Ok(())
            }
        }
    }

    /// Declare one secret. The same record declared twice is one
    /// secret — the rule that lets `FAL_KEY` (image_gen and video_gen)
    /// and `XAI_API_KEY` (the xai provider and the x_search toolset)
    /// be named from two places; the same name bound for a different
    /// target is the caller's contradiction
    /// ([`PrepareError::Contradiction`]).
    pub fn declare(&mut self, secret: Secret) -> Result<(), PrepareError> {
        match self.secrets.iter().find(|held| held.name == secret.name) {
            Some(held) if held.target != secret.target => {
                Err(PrepareError::Contradiction { variable: secret.name })
            }
            Some(_) => Ok(()),
            None => {
                self.secrets.push(secret);
                Ok(())
            }
        }
    }

    /// What the registration answers: every secret the plan takes,
    /// by name, to the mode its target implies, in the order declared.
    pub fn declaration(&self) -> Declaration {
        self.secrets
            .iter()
            .map(|secret| (secret.name.to_string(), secret.target.mode()))
            .collect()
    }
}
