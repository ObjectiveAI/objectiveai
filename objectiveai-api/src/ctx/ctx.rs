//! Request context containing per-request state and caches.

use dashmap::DashMap;
use futures::future::Shared;
use std::sync::Arc;

/// Per-request context containing user-specific state and deduplication caches.
///
/// The context is generic over `CTXEXT`, allowing custom extensions for
/// different deployment scenarios (e.g., different BYOK providers).
///
/// # Caches
///
/// The caches deduplicate concurrent fetches for the same resource within a request.
/// When multiple parts of a request need the same ensemble or ensemble LLM,
/// only one fetch is performed and the result is shared.
#[derive(Debug)]
pub struct Context<CTXEXT> {
    /// Custom context extension (e.g., for BYOK keys).
    pub ext: Arc<CTXEXT>,
    /// Multiplier applied to costs for this request.
    pub cost_multiplier: rust_decimal::Decimal,
    /// Cache for ensemble fetches, keyed by ensemble ID.
    pub ensemble_cache: Arc<
        DashMap<
            String,
            Shared<
                tokio::sync::oneshot::Receiver<
                    Result<
                        Option<(objectiveai::ensemble::Ensemble, u64)>,
                        objectiveai::error::ResponseError,
                    >,
                >,
            >,
        >,
    >,
    /// Cache for ensemble LLM fetches, keyed by ensemble LLM ID.
    pub ensemble_llm_cache: Arc<
        DashMap<
            String,
            Shared<
                tokio::sync::oneshot::Receiver<
                    Result<
                        Option<(objectiveai::ensemble_llm::EnsembleLlm, u64)>,
                        objectiveai::error::ResponseError,
                    >,
                >,
            >,
        >,
    >,
}

impl<CTXEXT> Clone for Context<CTXEXT> {
    fn clone(&self) -> Self {
        Self {
            ext: self.ext.clone(),
            cost_multiplier: self.cost_multiplier,
            ensemble_cache: self.ensemble_cache.clone(),
            ensemble_llm_cache: self.ensemble_llm_cache.clone(),
        }
    }
}

impl<CTXEXT> Context<CTXEXT> {
    /// Creates a new context with the given extension and cost multiplier.
    pub fn new(ext: Arc<CTXEXT>, cost_multiplier: rust_decimal::Decimal) -> Self {
        Self {
            ext,
            cost_multiplier,
            ensemble_cache: Arc::new(DashMap::new()),
            ensemble_llm_cache: Arc::new(DashMap::new()),
        }
    }
}
