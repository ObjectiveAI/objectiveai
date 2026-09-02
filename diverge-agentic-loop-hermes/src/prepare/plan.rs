//! What the agent adds up to, before anything is fetched or written.

use std::collections::BTreeMap;

use super::{Ask, PrepareError};

/// The accumulator the provider and the toolsets write into: the
/// environment, the config's inputs, the resources to fetch, and
/// the one file whose content the request carries inline.
#[derive(Debug, Default)]
pub struct Plan {
    /// The gateway's environment: the harness's variables only.
    pub env: BTreeMap<String, String>,
    /// `model.provider` — the provider's own id, as its marker
    /// serializes.
    pub provider: String,
    /// `model.default` — the model, in the provider's naming.
    pub model: String,
    /// `model.api_key` — the `custom` provider's, when it has one.
    pub api_key: Option<String>,
    /// `web.search_backend`, when a search slot names one.
    pub search_backend: Option<&'static str>,
    /// `web.extract_backend`, when the extract slot names one.
    pub extract_backend: Option<&'static str>,
    /// `browser.cloud_provider: browserbase`, when the browser's remote
    /// is Browserbase — pinned, so Hermes's auto-detect walk (Browser
    /// Use first) never enters into it.
    pub browserbase: bool,
    /// `tts.provider: elevenlabs`, when its key is present.
    pub tts_elevenlabs: bool,
    /// `image_gen.provider: fal`, when the toolset is present.
    pub image_gen: bool,
    /// `video_gen.provider: fal`, when the toolset is present.
    pub video_gen: bool,
    /// `platform_toolsets.api_server`: the toolsets exposed, by
    /// Hermes's own configurable-key names, in its order.
    pub toolsets: Vec<&'static str>,
    /// The resources to fetch, all at once, before any file is
    /// written.
    pub asks: Vec<Ask>,
    /// Vertex's service-account document, verbatim, to be written
    /// to the file `VERTEX_CREDENTIALS_PATH` names.
    pub vertex: Option<String>,
}

impl Plan {
    /// An empty plan for one model.
    pub fn new(model: String) -> Self {
        Plan {
            model,
            ..Plan::default()
        }
    }

    /// Set one environment variable. Setting it again to the SAME
    /// value is fine; to a different one is the caller's
    /// contradiction ([`PrepareError::Contradiction`]) — the rule
    /// that makes `FAL_KEY` (image_gen and video_gen) and
    /// `XAI_API_KEY` (the xai provider and the x_search toolset)
    /// agree with themselves.
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
}
