//! Wrapper around [`objectiveai_sdk::HttpClient`] that injects per-request
//! authorization headers from the request context.

use crate::ctx;
use std::sync::Arc;

/// ObjectiveAI HTTP client that injects per-request authorization from context.
///
/// Stores the same configuration as [`objectiveai_sdk::HttpClient`]. The per-request
/// authorization fields are populated via [`with_authorization`](Self::with_authorization).
#[derive(Debug, Clone)]
pub struct Client {
    pub http_client: reqwest::Client,
    pub address: String,
    pub authorization: Option<String>,
    pub user_agent: String,
    pub x_title: String,
    pub http_referer: String,
    pub x_github_authorization: Option<Arc<String>>,
    pub x_openrouter_authorization: Option<Arc<String>>,
    pub x_mcp_authorization: Option<Arc<std::collections::HashMap<String, String>>>,
}

impl Client {
    pub fn new(
        http_client: reqwest::Client,
        address: String,
        authorization: Option<String>,
        user_agent: String,
        x_title: String,
        http_referer: String,
        x_github_authorization: Option<Arc<String>>,
        x_openrouter_authorization: Option<Arc<String>>,
        x_mcp_authorization: Option<Arc<std::collections::HashMap<String, String>>>,
    ) -> Self {
        Self {
            http_client,
            address,
            authorization,
            user_agent,
            x_title,
            http_referer,
            x_github_authorization,
            x_openrouter_authorization,
            x_mcp_authorization,
        }
    }

    /// Creates an [`objectiveai_sdk::HttpClient`] with authorization headers
    /// populated from the request context.
    pub async fn with_authorization<CTXEXT: ctx::ContextExt>(
        &self,
        ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
    ) -> objectiveai_sdk::HttpClient {
        let (
            ctx_github_authorization,
            ctx_openrouter_authorization,
            ctx_mcp_authorization,
        ) = tokio::join!(
            ctx.github_authorization(),
            ctx.upstream_authorization(objectiveai_sdk::agent::Upstream::Openrouter),
            ctx.mcp_authorization(),
        );

        let authorization = match ctx.objectiveai_authorization() {
            Some(token) => Some(token.clone()),
            None => self.authorization.as_ref().map(|k| Arc::new(k.clone())),
        };

        objectiveai_sdk::HttpClient {
            http_client: self.http_client.clone(),
            address: self.address.clone(),
            authorization,
            user_agent: Some(self.user_agent.clone()),
            x_title: Some(self.x_title.clone()),
            http_referer: Some(self.http_referer.clone()),
            x_github_authorization: ctx_github_authorization
                .or_else(|| self.x_github_authorization.clone()),
            x_openrouter_authorization: ctx_openrouter_authorization
                .or_else(|| self.x_openrouter_authorization.clone()),
            x_mcp_authorization: ctx_mcp_authorization
                .or_else(|| self.x_mcp_authorization.clone()),
            // The api no longer reads or forwards viewer headers; the
            // SDK HttpClient keeps the fields for other consumers.
            x_viewer_signature: None,
            x_viewer_address: None,
            agent_instance_hierarchy: ctx.agent_instance_hierarchy().map(|s| Arc::new(s.to_string())),
            // No MCP session id surfaces through the per-request
            // `ctx` here. The api server's outgoing-MCP path stamps
            // its own session id when relevant; this http client is
            // for upstream LLM/SDK calls.
            mcp_session_id: None,
        }
    }
}
