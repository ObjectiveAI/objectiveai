use crate::chat;
use dashmap::DashMap;
use futures::future::Shared;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait ContextExt {
    async fn get_byok(
        &self,
        upstream: chat::completions::upstream::Upstream,
    ) -> Result<Option<String>, objectiveai::error::ResponseError>;
}

#[derive(Debug)]
pub struct Context<CTXEXT> {
    pub ext: Arc<CTXEXT>,
    pub cost_multiplier: rust_decimal::Decimal,
    pub ensemble_cache: Arc<
        DashMap<
            String,
            Shared<
                tokio::sync::oneshot::Receiver<
                    Result<
                        Option<objectiveai::ensemble::Ensemble>,
                        objectiveai::error::ResponseError,
                    >,
                >,
            >,
        >,
    >,
    pub ensemble_llm_cache: Arc<
        DashMap<
            String,
            Shared<
                tokio::sync::oneshot::Receiver<
                    Result<
                        Option<objectiveai::ensemble_llm::EnsembleLlm>,
                        objectiveai::error::ResponseError,
                    >,
                >,
            >,
        >,
    >,
    pub function_cache: Arc<
        DashMap<
            (
                String,         // owner
                String,         // repository
                Option<String>, // commit
            ),
            Shared<
                tokio::sync::oneshot::Receiver<
                    Result<
                        Option<objectiveai::functions::response::GetFunction>,
                        objectiveai::error::ResponseError,
                    >,
                >,
            >,
        >,
    >,
    pub profile_cache: Arc<
        DashMap<
            (
                String,         // owner
                String,         // repository
                Option<String>, // commit
            ),
            Shared<
                tokio::sync::oneshot::Receiver<
                    Result<
                        Option<objectiveai::functions::profiles::response::GetProfile>,
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
            function_cache: self.function_cache.clone(),
            profile_cache: self.profile_cache.clone(),
        }
    }
}

impl<CTXEXT> Context<CTXEXT> {
    pub fn new(
        ext: Arc<CTXEXT>,
        cost_multiplier: rust_decimal::Decimal,
    ) -> Self {
        Self {
            ext,
            cost_multiplier,
            ensemble_cache: Arc::new(DashMap::new()),
            ensemble_llm_cache: Arc::new(DashMap::new()),
            function_cache: Arc::new(DashMap::new()),
            profile_cache: Arc::new(DashMap::new()),
        }
    }
}
