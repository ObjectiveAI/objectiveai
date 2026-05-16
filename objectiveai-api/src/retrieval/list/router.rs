//! List router — merges results from all sources or filters by source.

use crate::ctx;
use objectiveai_sdk::error::ResponseError;
use std::sync::Arc;

/// Source filter for list endpoints.
#[derive(Debug, Clone, Copy)]
pub enum SourceFilter {
    All,
    Mock,
    Filesystem,
    Objectiveai,
}

/// Routes list operations to ObjectiveAI/Filesystem/Mock and merges results.
pub struct Router<O, F, M, CTXEXT> {
    pub objectiveai: Arc<O>,
    pub filesystem: Arc<F>,
    pub mock: Arc<M>,
    _ctxext: std::marker::PhantomData<CTXEXT>,
}

impl<O, F, M, CTXEXT> Router<O, F, M, CTXEXT> {
    pub fn new(objectiveai: Arc<O>, filesystem: Arc<F>, mock: Arc<M>) -> Self {
        Self { objectiveai, filesystem, mock, _ctxext: std::marker::PhantomData }
    }
}

/// No caching needed for list — results are not deduplicated.
impl<O, F, M, CTXEXT> Router<O, F, M, CTXEXT>
where
    O: super::Client<CTXEXT>,
    F: super::Client<CTXEXT>,
    M: super::Client<CTXEXT>,
    CTXEXT: Send + Sync + 'static,
{
    pub async fn list_agents(
        &self,
        ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        source: Option<SourceFilter>,
    ) -> Result<objectiveai_sdk::agent::response::ListAgentResponse, ResponseError> {
        use objectiveai_sdk::agent::response::ListAgentResponse;
        match source {
            Some(SourceFilter::Objectiveai) => self.objectiveai.list_agents(ctx).await,
            Some(SourceFilter::Filesystem) => self.filesystem.list_agents(ctx).await,
            Some(SourceFilter::Mock) => self.mock.list_agents(ctx).await,
            Some(SourceFilter::All) | None => {
                let (o, f, m) = futures::future::join3(
                    self.objectiveai.list_agents(ctx),
                    self.filesystem.list_agents(ctx),
                    self.mock.list_agents(ctx),
                ).await;
                let mut data = Vec::new();
                data.extend(o?.data);
                data.extend(f?.data);
                data.extend(m?.data);
                Ok(ListAgentResponse { data })
            }
        }
    }

    pub async fn list_swarms(
        &self,
        ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        source: Option<SourceFilter>,
    ) -> Result<objectiveai_sdk::swarm::response::ListSwarmResponse, ResponseError> {
        use objectiveai_sdk::swarm::response::ListSwarmResponse;
        match source {
            Some(SourceFilter::Objectiveai) => self.objectiveai.list_swarms(ctx).await,
            Some(SourceFilter::Filesystem) => self.filesystem.list_swarms(ctx).await,
            Some(SourceFilter::Mock) => self.mock.list_swarms(ctx).await,
            Some(SourceFilter::All) | None => {
                let (o, f, m) = futures::future::join3(
                    self.objectiveai.list_swarms(ctx),
                    self.filesystem.list_swarms(ctx),
                    self.mock.list_swarms(ctx),
                ).await;
                let mut data = Vec::new();
                data.extend(o?.data);
                data.extend(f?.data);
                data.extend(m?.data);
                Ok(ListSwarmResponse { data })
            }
        }
    }

    pub async fn list_functions(
        &self,
        ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        source: Option<SourceFilter>,
    ) -> Result<objectiveai_sdk::functions::response::ListFunctionResponse, ResponseError> {
        use objectiveai_sdk::functions::response::ListFunctionResponse;
        match source {
            Some(SourceFilter::Objectiveai) => self.objectiveai.list_functions(ctx).await,
            Some(SourceFilter::Filesystem) => self.filesystem.list_functions(ctx).await,
            Some(SourceFilter::Mock) => self.mock.list_functions(ctx).await,
            Some(SourceFilter::All) | None => {
                let (o, f, m) = futures::future::join3(
                    self.objectiveai.list_functions(ctx),
                    self.filesystem.list_functions(ctx),
                    self.mock.list_functions(ctx),
                ).await;
                let mut data = Vec::new();
                data.extend(o?.data);
                data.extend(f?.data);
                data.extend(m?.data);
                Ok(ListFunctionResponse { data })
            }
        }
    }

    pub async fn list_profiles(
        &self,
        ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        source: Option<SourceFilter>,
    ) -> Result<objectiveai_sdk::functions::profiles::response::ListProfileResponse, ResponseError> {
        use objectiveai_sdk::functions::profiles::response::ListProfileResponse;
        match source {
            Some(SourceFilter::Objectiveai) => self.objectiveai.list_profiles(ctx).await,
            Some(SourceFilter::Filesystem) => self.filesystem.list_profiles(ctx).await,
            Some(SourceFilter::Mock) => self.mock.list_profiles(ctx).await,
            Some(SourceFilter::All) | None => {
                let (o, f, m) = futures::future::join3(
                    self.objectiveai.list_profiles(ctx),
                    self.filesystem.list_profiles(ctx),
                    self.mock.list_profiles(ctx),
                ).await;
                let mut data = Vec::new();
                data.extend(o?.data);
                data.extend(f?.data);
                data.extend(m?.data);
                Ok(ListProfileResponse { data })
            }
        }
    }

    pub async fn list_prompts(
        &self,
        ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        source: Option<SourceFilter>,
    ) -> Result<objectiveai_sdk::functions::inventions::prompts::response::ListPromptResponse, ResponseError> {
        use objectiveai_sdk::functions::inventions::prompts::response::ListPromptResponse;
        match source {
            Some(SourceFilter::Objectiveai) => self.objectiveai.list_prompts(ctx).await,
            Some(SourceFilter::Filesystem) => self.filesystem.list_prompts(ctx).await,
            Some(SourceFilter::Mock) => self.mock.list_prompts(ctx).await,
            Some(SourceFilter::All) | None => {
                let (o, f, m) = futures::future::join3(
                    self.objectiveai.list_prompts(ctx),
                    self.filesystem.list_prompts(ctx),
                    self.mock.list_prompts(ctx),
                ).await;
                let mut data = Vec::new();
                data.extend(o?.data);
                data.extend(f?.data);
                data.extend(m?.data);
                Ok(ListPromptResponse { data })
            }
        }
    }

    pub async fn list_function_profile_pairs(
        &self,
        ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
    ) -> Result<objectiveai_sdk::functions::response::ListFunctionProfilePairResponse, ResponseError> {
        self.objectiveai.list_function_profile_pairs(ctx).await
    }
}
