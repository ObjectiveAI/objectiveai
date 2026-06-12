//! Viewer HTTP client. Fire-and-forget event publisher with exponential
//! backoff retries.
//!
//! The client owns an unbounded mpsc channel + a background tokio task.
//! Each `send_*` method is synchronous — it serializes the request and
//! pushes it onto the channel; the background task POSTs to the
//! viewer's HTTP endpoint with retry. Callers don't await network I/O.
//!
//! This module is intentionally ctx-free. Per-request `address` and
//! `signature` overrides are passed explicitly to each `send_*` call.
//! Higher-level consumers (e.g. `objectiveai-api`) can wrap this client
//! and do per-request context resolution on top.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;

use super::request::{
    AgentCompletionCreateParams, AgentCompletionRequest,
    FunctionExecutionCreateParams, FunctionExecutionRequest,
    FunctionInventionRecursiveCreateParams, FunctionInventionRecursiveRequest,
    LaboratoryExecutionCreateParams, LaboratoryExecutionRequest, Request,
    ResponseError,
};

/// Resolved per-request override pair.
///
/// `None` on either field means "fall back to the [`Client`]'s
/// `default_*`".
#[derive(Debug, Clone)]
pub(super) struct ViewerData {
    pub address: Option<Arc<String>>,
    pub signature: Option<Arc<String>>,
}

/// The viewer HTTP client. Constructed once at process startup; all
/// `send_*` methods take `&self` so the client is shared across the
/// app via `Arc<Client>`.
pub struct Client {
    tx: mpsc::UnboundedSender<(ViewerData, Request)>,
    /// Background task handle. `flush(self)` drops `tx` and `.await`s
    /// this so every enqueued request lands (or exhausts its retry
    /// budget) before the consumer drops the runtime.
    handle: tokio::task::JoinHandle<()>,
    /// Fallback address used when a `send_*` call's `address` is
    /// `None`. Set from the constructor's `address` argument, or from
    /// `VIEWER_ADDRESS` if `address` is `None` and the `env` feature
    /// is enabled.
    pub default_address: Option<Arc<String>>,
    /// Fallback signature used when a `send_*` call's `signature` is
    /// `None`. Set from the constructor's `signature` argument, or
    /// from `VIEWER_SIGNATURE` if `signature` is `None` and the `env`
    /// feature is enabled.
    pub default_signature: Option<Arc<String>>,
}

impl Client {
    /// Construct a new client and spawn its background task.
    ///
    /// `address` / `signature` defaults: an explicit `Some(...)` wins.
    /// When `None`, and the `env` feature is enabled, falls back to
    /// the `VIEWER_ADDRESS` / `VIEWER_SIGNATURE` env vars. When both
    /// the argument and the env var are missing, the client has no
    /// default — every `send_*` call must supply its own `address`,
    /// or the request is dropped.
    ///
    /// Backoff parameters tune the [`backoff::ExponentialBackoff`]
    /// the background task uses for retry. `max_elapsed_time` caps
    /// how long the task keeps retrying a single failed request
    /// before giving up.
    pub fn new(
        http_client: reqwest::Client,
        address: Option<impl Into<String>>,
        signature: Option<impl Into<String>>,
        backoff_current_interval: Duration,
        backoff_initial_interval: Duration,
        backoff_randomization_factor: f64,
        backoff_multiplier: f64,
        backoff_max_interval: Duration,
        backoff_max_elapsed_time: Duration,
    ) -> Self {
        let default_address = match address {
            Some(s) => Some(Arc::new(s.into())),
            #[cfg(feature = "env")]
            None => std::env::var("VIEWER_ADDRESS").ok().map(Arc::new),
            #[cfg(not(feature = "env"))]
            None => None,
        };
        let default_signature = match signature {
            Some(s) => Some(Arc::new(s.into())),
            #[cfg(feature = "env")]
            None => std::env::var("VIEWER_SIGNATURE").ok().map(Arc::new),
            #[cfg(not(feature = "env"))]
            None => None,
        };

        let (tx, mut rx) = mpsc::unbounded_channel::<(ViewerData, Request)>();

        let bg_default_address = default_address.clone();
        let bg_default_signature = default_signature.clone();

        let handle = tokio::spawn(async move {
            while let Some((viewer_data, request)) = rx.recv().await {
                let (address, signature) = match viewer_data.address {
                    Some(addr) => (addr, viewer_data.signature),
                    None => match &bg_default_address {
                        Some(addr) => {
                            (addr.clone(), bg_default_signature.clone())
                        }
                        None => continue,
                    },
                };

                let url = match &request {
                    Request::AgentCompletion(_) => {
                        format!("{}/agent/completions", address)
                    }
                    Request::FunctionExecution(_) => {
                        format!("{}/functions/executions", address)
                    }
                    Request::FunctionInventionRecursive(_) => {
                        format!("{}/functions/inventions/recursive", address)
                    }
                    Request::LaboratoryExecution(_) => {
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
                                req = req
                                    .header("X-VIEWER-SIGNATURE", sig.as_str());
                            }

                            let response = req
                                .send()
                                .await
                                .map_err(backoff::Error::transient)?;

                            if response.status().is_success() {
                                Ok(())
                            } else {
                                Err(backoff::Error::transient(
                                    response.error_for_status().unwrap_err(),
                                ))
                            }
                        }
                    },
                )
                .await;
            }
        });

        Self {
            tx,
            handle,
            default_address,
            default_signature,
        }
    }

    /// Closes the channel and awaits the background task. After this
    /// returns, every enqueued request has either succeeded or
    /// exhausted its retry budget. Takes `self` so the
    /// can't-send-after-flush invariant is enforced by the type
    /// system.
    ///
    /// Intended for short-lived consumers (e.g. the cli) that must
    /// drain in-flight POSTs before dropping the tokio runtime. Long-
    /// lived consumers (api server) can rely on the natural drop
    /// behavior — when the `Client` drops, `tx` drops, the bg task's
    /// `rx.recv()` returns `None`, and the loop exits.
    pub async fn flush(self) {
        let Self { tx, handle, .. } = self;
        drop(tx);
        let _ = handle.await;
    }

    /// Internal helper. Pushes one `(ViewerData, Request)` onto the
    /// channel; the background task takes it from there.
    fn enqueue(
        &self,
        address: Option<Arc<String>>,
        signature: Option<Arc<String>>,
        request: Request,
    ) {
        let _ = self.tx.send((ViewerData { address, signature }, request));
    }

    pub fn send_agent_completion_begin(
        &self,
        address: Option<Arc<String>>,
        signature: Option<Arc<String>>,
        id: String,
        request: Arc<
            crate::agent::completions::request::AgentCompletionCreateParams,
        >,
    ) {
        self.enqueue(
            address,
            signature,
            Request::AgentCompletion(AgentCompletionRequest::Begin(
                AgentCompletionCreateParams { id, inner: request },
            )),
        );
    }

    pub fn send_agent_completion_continue(
        &self,
        address: Option<Arc<String>>,
        signature: Option<Arc<String>>,
        chunk: crate::agent::completions::response::streaming::AgentCompletionChunk,
    ) {
        self.enqueue(
            address,
            signature,
            Request::AgentCompletion(AgentCompletionRequest::Continue(chunk)),
        );
    }

    pub fn send_agent_completion_error(
        &self,
        address: Option<Arc<String>>,
        signature: Option<Arc<String>>,
        id: String,
        error: crate::error::ResponseError,
    ) {
        self.enqueue(
            address,
            signature,
            Request::AgentCompletion(AgentCompletionRequest::Error(
                ResponseError { id, inner: error },
            )),
        );
    }

    pub fn send_function_execution_begin(
        &self,
        address: Option<Arc<String>>,
        signature: Option<Arc<String>>,
        id: String,
        request: Arc<crate::functions::executions::request::FunctionExecutionCreateParams>,
    ) {
        self.enqueue(
            address,
            signature,
            Request::FunctionExecution(FunctionExecutionRequest::Begin(
                FunctionExecutionCreateParams { id, inner: request },
            )),
        );
    }

    pub fn send_function_execution_continue(
        &self,
        address: Option<Arc<String>>,
        signature: Option<Arc<String>>,
        chunk: crate::functions::executions::response::streaming::FunctionExecutionChunk,
    ) {
        self.enqueue(
            address,
            signature,
            Request::FunctionExecution(FunctionExecutionRequest::Continue(
                chunk,
            )),
        );
    }

    pub fn send_function_execution_error(
        &self,
        address: Option<Arc<String>>,
        signature: Option<Arc<String>>,
        id: String,
        error: crate::error::ResponseError,
    ) {
        self.enqueue(
            address,
            signature,
            Request::FunctionExecution(FunctionExecutionRequest::Error(
                ResponseError { id, inner: error },
            )),
        );
    }

    pub fn send_function_invention_recursive_begin(
        &self,
        address: Option<Arc<String>>,
        signature: Option<Arc<String>>,
        id: String,
        request: Arc<
            crate::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams,
        >,
    ) {
        self.enqueue(
            address,
            signature,
            Request::FunctionInventionRecursive(
                FunctionInventionRecursiveRequest::Begin(
                    FunctionInventionRecursiveCreateParams {
                        id,
                        inner: request,
                    },
                ),
            ),
        );
    }

    pub fn send_function_invention_recursive_continue(
        &self,
        address: Option<Arc<String>>,
        signature: Option<Arc<String>>,
        chunk: crate::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk,
    ) {
        self.enqueue(
            address,
            signature,
            Request::FunctionInventionRecursive(
                FunctionInventionRecursiveRequest::Continue(chunk),
            ),
        );
    }

    pub fn send_function_invention_recursive_error(
        &self,
        address: Option<Arc<String>>,
        signature: Option<Arc<String>>,
        id: String,
        error: crate::error::ResponseError,
    ) {
        self.enqueue(
            address,
            signature,
            Request::FunctionInventionRecursive(
                FunctionInventionRecursiveRequest::Error(ResponseError {
                    id,
                    inner: error,
                }),
            ),
        );
    }

    pub fn send_laboratory_execution_begin(
        &self,
        address: Option<Arc<String>>,
        signature: Option<Arc<String>>,
        id: String,
        request: Arc<crate::laboratories::executions::request::LaboratoryExecutionCreateParams>,
    ) {
        self.enqueue(
            address,
            signature,
            Request::LaboratoryExecution(LaboratoryExecutionRequest::Begin(
                LaboratoryExecutionCreateParams { id, inner: request },
            )),
        );
    }

    pub fn send_laboratory_execution_continue(
        &self,
        address: Option<Arc<String>>,
        signature: Option<Arc<String>>,
        chunk: crate::laboratories::executions::response::streaming::LaboratoryExecutionChunk,
    ) {
        self.enqueue(
            address,
            signature,
            Request::LaboratoryExecution(LaboratoryExecutionRequest::Continue(
                chunk,
            )),
        );
    }

    pub fn send_laboratory_execution_error(
        &self,
        address: Option<Arc<String>>,
        signature: Option<Arc<String>>,
        id: String,
        error: crate::error::ResponseError,
    ) {
        self.enqueue(
            address,
            signature,
            Request::LaboratoryExecution(LaboratoryExecutionRequest::Error(
                ResponseError { id, inner: error },
            )),
        );
    }
}
