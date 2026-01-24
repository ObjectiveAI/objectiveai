//! Chat completions client implementation.

use futures::{StreamExt, TryStreamExt};

use crate::{ctx, util::StreamOnce};
use std::{sync::Arc, time::Duration};

/// Generates a unique response ID for a chat completion.
pub fn response_id(created: u64) -> String {
    let uuid = uuid::Uuid::new_v4();
    format!("chtcpl-{}-{}", uuid.simple(), created)
}

/// Client for creating chat completions.
///
/// Handles Ensemble LLM fetching, upstream provider selection with fallbacks,
/// retry logic with exponential backoff, and usage tracking.
#[derive(Debug, Clone)]
pub struct Client<CTXEXT, FENSLLM, CUSG> {
    /// Caching fetcher for Ensemble LLM definitions.
    pub ensemble_llm_fetcher:
        Arc<crate::ensemble_llm::fetcher::CachingFetcher<CTXEXT, FENSLLM>>,
    /// Handler for tracking usage after completion.
    pub usage_handler: Arc<CUSG>,
    /// Client for communicating with upstream providers.
    pub upstream_client: super::upstream::Client,

    /// Current backoff interval for retry logic.
    pub backoff_current_interval: Duration,
    /// Initial backoff interval for retry logic.
    pub backoff_initial_interval: Duration,
    /// Randomization factor for backoff jitter.
    pub backoff_randomization_factor: f64,
    /// Multiplier for exponential backoff growth.
    pub backoff_multiplier: f64,
    /// Maximum backoff interval.
    pub backoff_max_interval: Duration,
    /// Maximum total time to spend on retries.
    pub backoff_max_elapsed_time: Duration,
}

impl<CTXEXT, FENSLLM, CUSG> Client<CTXEXT, FENSLLM, CUSG> {
    /// Creates a new chat completions client.
    pub fn new(
        ensemble_llm_fetcher: Arc<
            crate::ensemble_llm::fetcher::CachingFetcher<CTXEXT, FENSLLM>,
        >,
        usage_handler: Arc<CUSG>,
        upstream_client: super::upstream::Client,
        backoff_current_interval: Duration,
        backoff_initial_interval: Duration,
        backoff_randomization_factor: f64,
        backoff_multiplier: f64,
        backoff_max_interval: Duration,
        backoff_max_elapsed_time: Duration,
    ) -> Self {
        Self {
            ensemble_llm_fetcher,
            usage_handler,
            upstream_client,
            backoff_current_interval,
            backoff_initial_interval,
            backoff_randomization_factor,
            backoff_multiplier,
            backoff_max_interval,
            backoff_max_elapsed_time,
        }
    }
}

impl<CTXEXT, FENSLLM, CUSG> Client<CTXEXT, FENSLLM, CUSG>
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    FENSLLM:
        crate::ensemble_llm::fetcher::Fetcher<CTXEXT> + Send + Sync + 'static,
    CUSG: super::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
{
    /// Creates a unary chat completion, tracking usage after completion.
    ///
    /// Internally streams the response and aggregates chunks into a single response.
    pub async fn create_unary_for_chat_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<
            objectiveai::chat::completions::request::ChatCompletionCreateParams,
        >,
    ) -> Result<
        objectiveai::chat::completions::response::unary::ChatCompletion,
        super::Error,
    > {
        let mut aggregate: Option<
            objectiveai::chat::completions::response::streaming::ChatCompletionChunk,
        > = None;
        let mut stream = self
            .create_streaming_for_chat_handle_usage(ctx, request)
            .await?;
        while let Some(chunk) = stream.try_next().await? {
            match &mut aggregate {
                Some(aggregate) => aggregate.push(&chunk),
                None => {
                    aggregate = Some(chunk);
                }
            }
        }
        Ok(aggregate.unwrap().into())
    }

    /// Creates a streaming chat completion, tracking usage after the stream ends.
    pub async fn create_streaming_for_chat_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::chat::completions::request::ChatCompletionCreateParams>,
    ) -> Result<
        impl futures::Stream<
            Item = Result<
                objectiveai::chat::completions::response::streaming::ChatCompletionChunk,
                super::Error,
            >,
        > + Send
        + Unpin
        + 'static,
        super::Error,
    >{
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = tokio::spawn(async move {
            let mut aggregate: Option<
                objectiveai::chat::completions::response::streaming::ChatCompletionChunk,
            > = None;
            let mut error = false;
            let stream = match self
                .clone()
                .create_streaming_for_chat(ctx.clone(), request.clone())
                .await
            {
                Ok(stream) => stream,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            futures::pin_mut!(stream);
            while let Some(result) = stream.next().await {
                match &result {
                    Ok(chunk) => match &mut aggregate {
                        Some(aggregate) => aggregate.push(chunk),
                        None => {
                            aggregate = Some(chunk.clone());
                        }
                    },
                    Err(_) => {
                        error = true;
                    }
                }
                let _ = tx.send(result);
            }
            drop(stream);
            drop(tx);
            if !error {
                self.usage_handler
                    .handle_usage(ctx, Some(request), aggregate.unwrap().into())
                    .await;
            }
        });
        let mut stream =
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
        match stream.next().await {
            Some(Ok(chunk)) => Ok(StreamOnce::new(Ok(chunk)).chain(stream)),
            Some(Err(e)) => Err(e),
            None => unreachable!(),
        }
    }

    /// Creates a streaming completion for vector voting, tracking usage after the stream ends.
    ///
    /// Used internally by vector completions to generate LLM votes.
    pub async fn create_streaming_for_vector_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<
            objectiveai::vector::completions::request::VectorCompletionCreateParams,
        >,
        vector_pfx_indices: Vec<Arc<Vec<(String, usize)>>>,
        ensemble_llm: objectiveai::ensemble_llm::EnsembleLlmWithFallbacksAndCount,
    ) -> Result<
        impl futures::Stream<
            Item = Result<
                objectiveai::chat::completions::response::streaming::ChatCompletionChunk,
                super::Error,
            >,
        > + Send
        + Unpin
        + 'static,
        super::Error,
    >{
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = tokio::spawn(async move {
            let mut aggregate: Option<
                objectiveai::chat::completions::response::streaming::ChatCompletionChunk,
            > = None;
            let mut error = false;
            let stream = match self
                .clone()
                .create_streaming_for_vector(
                    ctx.clone(),
                    request,
                    vector_pfx_indices,
                    ensemble_llm,
                )
                .await
            {
                Ok(stream) => stream,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            futures::pin_mut!(stream);
            while let Some(result) = stream.next().await {
                match &result {
                    Ok(chunk) => match &mut aggregate {
                        Some(aggregate) => aggregate.push(chunk),
                        None => {
                            aggregate = Some(chunk.clone());
                        }
                    },
                    Err(_) => {
                        error = true;
                    }
                }
                let _ = tx.send(result);
            }
            drop(stream);
            drop(tx);
            if !error {
                self.usage_handler
                    .handle_usage(ctx, None, aggregate.unwrap().into())
                    .await;
            }
        });
        let mut stream =
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
        match stream.next().await {
            Some(Ok(chunk)) => Ok(StreamOnce::new(Ok(chunk)).chain(stream)),
            Some(Err(e)) => Err(e),
            None => unreachable!(),
        }
    }
}

impl<CTXEXT, FENSLLM, CUSG> Client<CTXEXT, FENSLLM, CUSG>
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    FENSLLM:
        crate::ensemble_llm::fetcher::Fetcher<CTXEXT> + Send + Sync + 'static,
{
    /// Creates a streaming chat completion without usage tracking.
    ///
    /// Handles model validation, Ensemble LLM fetching, fallback logic,
    /// and retry with exponential backoff.
    pub async fn create_streaming_for_chat(
        &self,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::chat::completions::request::ChatCompletionCreateParams>,
    ) -> Result<
        impl futures::Stream<
            Item = Result<
                objectiveai::chat::completions::response::streaming::ChatCompletionChunk,
                super::Error,
            >,
        > + Send
        + Unpin
        + 'static,
        super::Error,
    >{
        // timestamp and identify the completion
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let response_id = response_id(created);

        // validate models IDs
        if let objectiveai::chat::completions::request::Model::Id(id) =
            &request.model
        {
            if id.len() != 22 {
                return Err(super::Error::InvalidEnsembleLlm(format!(
                    "invalid ID: {}",
                    id
                )));
            }
        }
        if let Some(models) = &request.models {
            for model in models {
                if let objectiveai::chat::completions::request::Model::Id(id) =
                    model
                {
                    if id.len() != 22 {
                        return Err(super::Error::InvalidEnsembleLlm(format!(
                            "invalid ID: {}",
                            id
                        )));
                    }
                }
            }
        }

        // collect all Ensemble LLMs
        let mut models = Vec::with_capacity(
            1 + request.models.as_ref().map(Vec::len).unwrap_or_default(),
        );
        models.push(&request.model);
        if let Some(request_models) = &request.models {
            models.extend(request_models.iter());
        }

        // spawn fetches for all Ensemble LLMs
        self.ensemble_llm_fetcher.spawn_fetches(
            ctx.clone(),
            models.iter().filter_map(|model| {
                if let objectiveai::chat::completions::request::Model::Id(id) =
                    model
                {
                    Some(id.as_str())
                } else {
                    None
                }
            }),
        );

        // backoff and timeouts
        let backoff = backoff::ExponentialBackoff {
            current_interval: self.backoff_current_interval,
            initial_interval: self.backoff_initial_interval,
            randomization_factor: self.backoff_randomization_factor,
            multiplier: self.backoff_multiplier,
            max_interval: self.backoff_max_interval,
            start_time: std::time::Instant::now(),
            max_elapsed_time: Some(
                request
                    .backoff_max_elapsed_time
                    .map(|ms| ms.min(600_000)) // at most 10 minutes
                    .map(Duration::from_millis)
                    .unwrap_or(self.backoff_max_elapsed_time),
            ),
            clock: backoff::SystemClock::default(),
        };
        let first_chunk_timeout = Duration::from_millis(
            request
                .first_chunk_timeout
                .unwrap_or(10_000) // default 10 seconds
                .min(10_000) // at least 10 seconds
                .max(120_000), // at most 2 minutes
        );
        let other_chunk_timeout = Duration::from_millis(
            request
                .other_chunk_timeout
                .unwrap_or(40_000) // default 40 seconds
                .min(40_000) // at least 40 seconds
                .max(120_000), // at most 2 minutes
        );

        // try each model in order
        backoff::future::retry(backoff, || async {
            let mut errors = Vec::new();
            for model in &models {
                // fetch or validate Ensemble LLM
                let ensemble_llm = Arc::new(match model {
                    objectiveai::chat::completions::request::Model::Id(id) => {
                        match self
                            .ensemble_llm_fetcher
                            .fetch(ctx.clone(), id)
                            .await
                        {
                            Ok(Some((ensemble_llm, _))) => ensemble_llm,
                            Ok(None) => {
                                errors.push(super::Error::EnsembleLlmNotFound);
                                continue;
                            }
                            Err(e) => {
                                errors.push(super::Error::FetchEnsembleLlm(e));
                                continue;
                            }
                        }
                    }
                    objectiveai::chat::completions::request::Model::Provided(ensemble_llm_base) => {
                        match ensemble_llm_base.clone().try_into() {
                            Ok(ensemble_llm) => ensemble_llm,
                            Err(msg) => {
                                errors.push(super::Error::InvalidEnsembleLlm(msg));
                                continue;
                            }
                        }
                    }
                });
                // try to create streaming completion
                match self.upstream_client.create_streaming(
                    ctx.clone(),
                    response_id.clone(),
                    first_chunk_timeout,
                    other_chunk_timeout,
                    ensemble_llm,
                    super::upstream::Params::Chat {
                        request: request.clone(),
                    },
                ).await {
                    Ok(Some(stream)) => {
                        return Ok(stream.map_err(super::Error::UpstreamError));
                    }
                    Ok(None) => {}
                    Err(e) => {
                        errors.push(super::Error::UpstreamError(e));
                    }
                }
            }
            if errors.is_empty() {
                Err(backoff::Error::permanent(super::Error::NoUpstreamsFound))
            } else {
                Err(backoff::Error::transient(super::Error::MultipleErrors(
                    errors,
                )))
            }
        })
        .await
    }

    /// Creates a streaming completion for vector voting without usage tracking.
    ///
    /// Used internally by vector completions. Handles fallback logic
    /// and retry with exponential backoff.
    pub async fn create_streaming_for_vector(
        &self,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<
            objectiveai::vector::completions::request::VectorCompletionCreateParams,
        >,
        vector_pfx_indices: Vec<Arc<Vec<(String, usize)>>>,
        ensemble_llm: objectiveai::ensemble_llm::EnsembleLlmWithFallbacksAndCount,
    ) -> Result<
        impl futures::Stream<
            Item = Result<
                objectiveai::chat::completions::response::streaming::ChatCompletionChunk,
                super::Error,
            >,
        > + Send
        + Unpin
        + 'static,
        super::Error,
    >{
        // timestamp and identify the completion
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let response_id = response_id(created);

        // collect all Ensemble LLMs
        let mut models = Vec::with_capacity(
            1 + ensemble_llm
                .fallbacks
                .as_ref()
                .map(Vec::len)
                .unwrap_or_default(),
        );
        models.push(Arc::new(ensemble_llm.inner));
        if let Some(fallbacks) = ensemble_llm.fallbacks {
            models.extend(fallbacks.into_iter().map(Arc::new));
        }

        // backoff and timeouts
        let backoff = backoff::ExponentialBackoff {
            current_interval: self.backoff_current_interval,
            initial_interval: self.backoff_initial_interval,
            randomization_factor: self.backoff_randomization_factor,
            multiplier: self.backoff_multiplier,
            max_interval: self.backoff_max_interval,
            start_time: std::time::Instant::now(),
            max_elapsed_time: Some(
                request
                    .backoff_max_elapsed_time
                    .map(|ms| ms.min(600_000)) // at most 10 minutes
                    .map(Duration::from_millis)
                    .unwrap_or(self.backoff_max_elapsed_time),
            ),
            clock: backoff::SystemClock::default(),
        };
        let first_chunk_timeout = Duration::from_millis(
            request
                .first_chunk_timeout
                .unwrap_or(10_000) // default 10 seconds
                .min(10_000) // at least 10 seconds
                .max(120_000), // at most 2 minutes
        );
        let other_chunk_timeout = Duration::from_millis(
            request
                .other_chunk_timeout
                .unwrap_or(40_000) // default 40 seconds
                .min(40_000) // at least 40 seconds
                .max(120_000), // at most 2 minutes
        );

        // try each model in order
        backoff::future::retry(backoff, || async {
            let mut errors = Vec::new();
            for (i, ensemble_llm) in models.iter().cloned().enumerate() {
                // try to create streaming completion
                match self
                    .upstream_client
                    .create_streaming(
                        ctx.clone(),
                        response_id.clone(),
                        first_chunk_timeout,
                        other_chunk_timeout,
                        ensemble_llm.clone(),
                        super::upstream::Params::Vector {
                            request: request.clone(),
                            vector_pfx_indices: vector_pfx_indices[i].clone(),
                        },
                    )
                    .await
                {
                    Ok(Some(stream)) => {
                        return Ok(stream.map_err(super::Error::UpstreamError));
                    }
                    Ok(None) => {}
                    Err(e) => {
                        errors.push(super::Error::UpstreamError(e));
                    }
                }
            }
            if errors.is_empty() {
                Err(backoff::Error::permanent(super::Error::NoUpstreamsFound))
            } else {
                Err(backoff::Error::transient(super::Error::MultipleErrors(
                    errors,
                )))
            }
        })
        .await
    }
}
