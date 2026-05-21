//! Context-aware wrapper around [`objectiveai_sdk::http::viewer::Client`].
//!
//! The SDK client is ctx-free — every `send_*` method takes resolved
//! `(address, signature)` arguments. This wrapper carries the api
//! server's per-request `Context<CTXEXT>`-resolution on top: each
//! `send_*` method spawns a task that awaits `ctx.viewer_address()` +
//! `ctx.viewer_signature()`, then calls the inner SDK client's
//! matching method with the resolved values.
//!
//! Constructor + send-method signatures match the pre-move shape so
//! call sites in `crate::agent::completions::client` /
//! `crate::functions::executions::client` / etc. don't change.

use std::sync::Arc;
use std::time::Duration;

use crate::ctx;

pub struct Client<CTXEXT> {
    inner: Arc<objectiveai_sdk::http::viewer::Client>,
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
        let inner = Arc::new(objectiveai_sdk::http::viewer::Client::new(
            http_client,
            address,
            signature,
            backoff_current_interval,
            backoff_initial_interval,
            backoff_randomization_factor,
            backoff_multiplier,
            backoff_max_interval,
            backoff_max_elapsed_time,
        ));
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Spawn a task that resolves `(viewer_address, viewer_signature)`
    /// from the context and invokes `f` with the resolved values. `f`
    /// is the SDK client send-method to call. The send is dropped
    /// silently if `ctx.viewer_address()` returns `None` and the
    /// inner client has no default address (matching the pre-move
    /// behavior).
    fn dispatch<F>(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient + 'static>,
        f: F,
    ) where
        F: FnOnce(
                Arc<objectiveai_sdk::http::viewer::Client>,
                Option<Arc<String>>,
                Option<Arc<String>>,
            ) + Send
            + 'static,
    {
        let inner = self.inner.clone();
        tokio::spawn(async move {
            let addr_fut = ctx.viewer_address();
            let sig_fut = ctx.viewer_signature();
            tokio::pin!(addr_fut);
            tokio::pin!(sig_fut);

            let (address, signature) = tokio::select! {
                biased;
                addr = &mut addr_fut => {
                    let sig = sig_fut.await;
                    (addr, sig)
                }
                sig = &mut sig_fut => {
                    let addr = addr_fut.await;
                    (addr, sig)
                }
            };

            f(inner, address, signature);
        });
    }

    pub fn send_agent_completion_begin(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient + 'static>,
        id: String,
        request: Arc<objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams>,
    ) {
        self.dispatch(ctx, move |inner, address, signature| {
            inner.send_agent_completion_begin(address, signature, id, request);
        });
    }

    pub fn send_agent_completion_continue(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient + 'static>,
        chunk: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
    ) {
        self.dispatch(ctx, move |inner, address, signature| {
            inner.send_agent_completion_continue(address, signature, chunk);
        });
    }

    pub fn send_agent_completion_error(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient + 'static>,
        id: String,
        error: &crate::agent::completions::Error,
    ) {
        let err = objectiveai_sdk::error::ResponseError::from(error);
        self.dispatch(ctx, move |inner, address, signature| {
            inner.send_agent_completion_error(address, signature, id, err);
        });
    }

    pub fn send_function_execution_begin(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient + 'static>,
        id: String,
        request: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
    ) {
        self.dispatch(ctx, move |inner, address, signature| {
            inner.send_function_execution_begin(address, signature, id, request);
        });
    }

    pub fn send_function_execution_continue(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient + 'static>,
        chunk: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk,
    ) {
        self.dispatch(ctx, move |inner, address, signature| {
            inner.send_function_execution_continue(address, signature, chunk);
        });
    }

    pub fn send_function_execution_error(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient + 'static>,
        id: String,
        error: &crate::functions::executions::Error,
    ) {
        let err = objectiveai_sdk::error::ResponseError::from(error);
        self.dispatch(ctx, move |inner, address, signature| {
            inner.send_function_execution_error(address, signature, id, err);
        });
    }

    pub fn send_function_invention_recursive_begin(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient + 'static>,
        id: String,
        request: Arc<objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams>,
    ) {
        self.dispatch(ctx, move |inner, address, signature| {
            inner.send_function_invention_recursive_begin(address, signature, id, request);
        });
    }

    pub fn send_function_invention_recursive_continue(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient + 'static>,
        chunk: objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk,
    ) {
        self.dispatch(ctx, move |inner, address, signature| {
            inner.send_function_invention_recursive_continue(address, signature, chunk);
        });
    }

    pub fn send_function_invention_recursive_error(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient + 'static>,
        id: String,
        error: &crate::functions::inventions::recursive::Error,
    ) {
        let err = objectiveai_sdk::error::ResponseError::from(error);
        self.dispatch(ctx, move |inner, address, signature| {
            inner.send_function_invention_recursive_error(address, signature, id, err);
        });
    }

    pub fn send_laboratory_execution_begin(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient + 'static>,
        id: String,
        request: Arc<objectiveai_sdk::laboratories::executions::request::LaboratoryExecutionCreateParams>,
    ) {
        self.dispatch(ctx, move |inner, address, signature| {
            inner.send_laboratory_execution_begin(address, signature, id, request);
        });
    }

    pub fn send_laboratory_execution_continue(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient + 'static>,
        chunk: objectiveai_sdk::laboratories::executions::response::streaming::LaboratoryExecutionChunk,
    ) {
        self.dispatch(ctx, move |inner, address, signature| {
            inner.send_laboratory_execution_continue(address, signature, chunk);
        });
    }

    pub fn send_laboratory_execution_error(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient + 'static>,
        id: String,
        error: objectiveai_sdk::error::ResponseError,
    ) {
        self.dispatch(ctx, move |inner, address, signature| {
            inner.send_laboratory_execution_error(address, signature, id, error);
        });
    }
}
