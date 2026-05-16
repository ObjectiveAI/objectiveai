use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::ctx;

/// Resolved viewer context data (address + signature).
struct ViewerData {
    address: Option<Arc<String>>,
    signature: Option<Arc<String>>,
}

pub struct Client<CTXEXT> {
    tx: mpsc::UnboundedSender<(ViewerData, super::request::Request)>,
    default_address: Option<Arc<String>>,
    default_signature: Option<Arc<String>>,
    _marker: std::marker::PhantomData<CTXEXT>,
}

impl<CTXEXT: ctx::ContextExt + Send + Sync + 'static> Client<CTXEXT> {
    pub fn new(
        http_client: reqwest::Client,
        address: Option<String>,
        signature: Option<String>,
        backoff_current_interval: Duration,
        backoff_initial_interval: Duration,
        backoff_randomization_factor: f64,
        backoff_multiplier: f64,
        backoff_max_interval: Duration,
        backoff_max_elapsed_time: Duration,
    ) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<(ViewerData, super::request::Request)>();

        let default_address = address.map(Arc::new);
        let default_signature = signature.map(Arc::new);

        let bg_default_address = default_address.clone();
        let bg_default_signature = default_signature.clone();

        tokio::spawn(async move {
            while let Some((viewer_data, request)) = rx.recv().await {
                let (address, signature) = match viewer_data.address {
                    Some(addr) => (addr, viewer_data.signature),
                    None => match &bg_default_address {
                        Some(addr) => (addr.clone(), bg_default_signature.clone()),
                        None => continue,
                    },
                };

                let url = match &request {
                    super::request::Request::AgentCompletion(_) => {
                        format!("{}/agent/completions", address)
                    }
                    super::request::Request::FunctionExecution(_) => {
                        format!("{}/functions/executions", address)
                    }
                    super::request::Request::FunctionInventionRecursive(_) => {
                        format!("{}/functions/inventions/recursive", address)
                    }
                    super::request::Request::LaboratoryExecution(_) => {
                        format!("{}/laboratories/executions", address)
                    }
                };

                let body = match serde_json::to_vec(&request) {
                    Ok(body) => body,
                    Err(_) => continue,
                };

                let _ = backoff::future::retry(
                    backoff::ExponentialBackoff {
                        current_interval: backoff_current_interval,
                        initial_interval: backoff_initial_interval,
                        randomization_factor: backoff_randomization_factor,
                        multiplier: backoff_multiplier,
                        max_interval: backoff_max_interval,
                        max_elapsed_time: Some(backoff_max_elapsed_time),
                        start_time: std::time::Instant::now(),
                        clock: backoff::SystemClock::default(),
                    },
                    || {
                        let http_client = &http_client;
                        let url = &url;
                        let body = &body;
                        let signature = &signature;
                        async move {
                            let mut req = http_client
                                .post(url.as_str())
                                .header("Content-Type", "application/json")
                                .body(body.clone());

                            if let Some(sig) = signature {
                                req = req.header("X-VIEWER-SIGNATURE", sig.as_str());
                            }

                            let response = req.send().await
                                .map_err(backoff::Error::transient)?;

                            if response.status().is_success() {
                                Ok(())
                            } else {
                                Err(backoff::Error::transient(
                                    response.error_for_status().unwrap_err()
                                ))
                            }
                        }
                    },
                ).await;
            }
        });

        Self { tx, default_address, default_signature, _marker: std::marker::PhantomData }
    }

    /// Resolves viewer data from the context and sends the request through the channel.
    /// The resolution is done in a spawned task to avoid blocking the caller.
    fn send_with_ctx(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: super::request::Request,
    ) {
        let tx = self.tx.clone();
        let default_address = self.default_address.clone();
        let default_signature = self.default_signature.clone();
        tokio::spawn(async move {
            let addr_fut = ctx.viewer_address();
            let sig_fut = ctx.viewer_signature();
            tokio::pin!(addr_fut);
            tokio::pin!(sig_fut);

            let (address, signature) = tokio::select! {
                biased;
                addr = &mut addr_fut => {
                    match addr {
                        Some(addr) => {
                            let sig = sig_fut.await;
                            (Some(addr), sig)
                        }
                        None => match &default_address {
                            Some(addr) => (Some(addr.clone()), default_signature.clone()),
                            None => return,
                        },
                    }
                }
                sig = &mut sig_fut => {
                    let addr = addr_fut.await;
                    match addr {
                        Some(addr) => (Some(addr), sig),
                        None => match &default_address {
                            Some(addr) => (Some(addr.clone()), default_signature.clone()),
                            None => return,
                        },
                    }
                }
            };

            let _ = tx.send((ViewerData { address, signature }, request));
        });
    }

    pub fn send_agent_completion_begin(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        id: String,
        request: Arc<objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams>,
    ) {
        self.send_with_ctx(ctx, super::request::Request::AgentCompletion(
            super::request::AgentCompletionRequest::Begin(super::request::AgentCompletionCreateParams {
                id,
                inner: request,
            }),
        ));
    }

    pub fn send_agent_completion_continue(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        chunk: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
    ) {
        self.send_with_ctx(ctx, super::request::Request::AgentCompletion(
            super::request::AgentCompletionRequest::Continue(chunk),
        ));
    }

    pub fn send_agent_completion_error(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        id: String,
        error: &crate::agent::completions::Error,
    ) {
        self.send_with_ctx(ctx, super::request::Request::AgentCompletion(
            super::request::AgentCompletionRequest::Error(super::request::ResponseError {
                id,
                inner: objectiveai_sdk::error::ResponseError::from(error),
            }),
        ));
    }

    pub fn send_function_execution_begin(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        id: String,
        request: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
    ) {
        self.send_with_ctx(ctx, super::request::Request::FunctionExecution(
            super::request::FunctionExecutionRequest::Begin(super::request::FunctionExecutionCreateParams {
                id,
                inner: request,
            }),
        ));
    }

    pub fn send_function_execution_continue(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        chunk: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk,
    ) {
        self.send_with_ctx(ctx, super::request::Request::FunctionExecution(
            super::request::FunctionExecutionRequest::Continue(chunk),
        ));
    }

    pub fn send_function_execution_error(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        id: String,
        error: &crate::functions::executions::Error,
    ) {
        self.send_with_ctx(ctx, super::request::Request::FunctionExecution(
            super::request::FunctionExecutionRequest::Error(super::request::ResponseError {
                id,
                inner: objectiveai_sdk::error::ResponseError::from(error),
            }),
        ));
    }

    pub fn send_function_invention_recursive_begin(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        id: String,
        request: Arc<objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams>,
    ) {
        self.send_with_ctx(ctx, super::request::Request::FunctionInventionRecursive(
            super::request::FunctionInventionRecursiveRequest::Begin(super::request::FunctionInventionRecursiveCreateParams {
                id,
                inner: request,
            }),
        ));
    }

    pub fn send_function_invention_recursive_continue(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        chunk: objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk,
    ) {
        self.send_with_ctx(ctx, super::request::Request::FunctionInventionRecursive(
            super::request::FunctionInventionRecursiveRequest::Continue(chunk),
        ));
    }

    pub fn send_laboratory_execution_begin(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        id: String,
        request: Arc<objectiveai_sdk::laboratories::executions::request::LaboratoryExecutionCreateParams>,
    ) {
        self.send_with_ctx(ctx, super::request::Request::LaboratoryExecution(
            super::request::LaboratoryExecutionRequest::Begin(super::request::LaboratoryExecutionCreateParams {
                id,
                inner: request,
            }),
        ));
    }

    pub fn send_laboratory_execution_continue(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        chunk: objectiveai_sdk::laboratories::executions::response::streaming::LaboratoryExecutionChunk,
    ) {
        self.send_with_ctx(ctx, super::request::Request::LaboratoryExecution(
            super::request::LaboratoryExecutionRequest::Continue(chunk),
        ));
    }

    pub fn send_laboratory_execution_error(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        id: String,
        error: objectiveai_sdk::error::ResponseError,
    ) {
        self.send_with_ctx(ctx, super::request::Request::LaboratoryExecution(
            super::request::LaboratoryExecutionRequest::Error(super::request::ResponseError {
                id,
                inner: error,
            }),
        ));
    }

    pub fn send_function_invention_recursive_error(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        id: String,
        error: &crate::functions::inventions::recursive::Error,
    ) {
        self.send_with_ctx(ctx, super::request::Request::FunctionInventionRecursive(
            super::request::FunctionInventionRecursiveRequest::Error(super::request::ResponseError {
                id,
                inner: objectiveai_sdk::error::ResponseError::from(error),
            }),
        ));
    }
}
