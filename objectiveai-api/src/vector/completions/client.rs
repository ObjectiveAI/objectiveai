//! Vector completion client implementation.

use crate::{
    chat, ctx,
    util::{ChoiceIndexer, StreamOnce},
};
use futures::{FutureExt, Stream, StreamExt, TryStreamExt};
use rand::Rng;
use rust_decimal::Decimal;
use std::{collections::HashMap, sync::Arc, time};

/// Generates a unique response ID for a vector completion.
pub fn response_id(created: u64) -> String {
    let uuid = uuid::Uuid::new_v4();
    format!("vctcpl-{}-{}", uuid.simple(), created)
}

/// Client for creating vector completions.
///
/// Orchestrates multiple LLM chat completions to vote on response options,
/// combining their votes using weights to produce final scores.
pub struct Client<CTXEXT, FENSLLM, CUSG, FENS, FVVOTE, FCVOTE, VUSG> {
    /// The underlying chat completion client.
    pub chat_client: Arc<chat::completions::Client<CTXEXT, FENSLLM, CUSG>>,
    /// Fetcher for Ensemble definitions.
    pub ensemble_fetcher:
        Arc<crate::ensemble::fetcher::CachingFetcher<CTXEXT, FENS>>,
    /// Fetcher for votes from historical completions.
    pub completion_votes_fetcher: Arc<FVVOTE>,
    /// Fetcher for votes from the global cache.
    pub cache_vote_fetcher: Arc<FCVOTE>,
    /// Handler for usage tracking.
    pub usage_handler: Arc<VUSG>,
}

impl<CTXEXT, FENSLLM, CUSG, FENS, FVVOTE, FCVOTE, VUSG>
    Client<CTXEXT, FENSLLM, CUSG, FENS, FVVOTE, FCVOTE, VUSG>
{
    /// Creates a new vector completion client.
    pub fn new(
        chat_client: Arc<chat::completions::Client<CTXEXT, FENSLLM, CUSG>>,
        ensemble_fetcher: Arc<
            crate::ensemble::fetcher::CachingFetcher<CTXEXT, FENS>,
        >,
        completion_votes_fetcher: Arc<FVVOTE>,
        cache_vote_fetcher: Arc<FCVOTE>,
        usage_handler: Arc<VUSG>,
    ) -> Self {
        Self {
            chat_client,
            ensemble_fetcher,
            completion_votes_fetcher,
            cache_vote_fetcher,
            usage_handler,
        }
    }
}

impl<CTXEXT, FENSLLM, CUSG, FENS, FVVOTE, FCVOTE, VUSG>
    Client<CTXEXT, FENSLLM, CUSG, FENS, FVVOTE, FCVOTE, VUSG>
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    FENSLLM: crate::ensemble_llm::fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    CUSG: chat::completions::usage_handler::UsageHandler<CTXEXT>
        + Send
        + Sync
        + 'static,
    FENS: crate::ensemble::fetcher::Fetcher<CTXEXT> + Send + Sync + 'static,
    FVVOTE: super::completion_votes_fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    FCVOTE: super::cache_vote_fetcher::Fetcher<CTXEXT> + Send + Sync + 'static,
    VUSG: super::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
{
    /// Creates a unary (non-streaming) vector completion with usage tracking.
    ///
    /// Collects all streaming chunks into a single response.
    pub async fn create_unary_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::vector::completions::request::VectorCompletionCreateParams>,
    ) -> Result<
        objectiveai::vector::completions::response::unary::VectorCompletion,
        super::Error,
    > {
        let mut aggregate: Option<
            objectiveai::vector::completions::response::streaming::VectorCompletionChunk,
        > = None;
        let mut stream =
            self.create_streaming_handle_usage(ctx, request).await?;
        while let Some(chunk) = stream.next().await {
            match &mut aggregate {
                Some(aggregate) => aggregate.push(&chunk),
                None => {
                    aggregate = Some(chunk);
                }
            }
        }
        Ok(aggregate.unwrap().into())
    }

    /// Creates a streaming vector completion with usage tracking.
    ///
    /// Spawns a background task to track usage after the stream completes.
    pub async fn create_streaming_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::vector::completions::request::VectorCompletionCreateParams>,
    ) -> Result<
        impl Stream<Item = objectiveai::vector::completions::response::streaming::VectorCompletionChunk>
        + Send
        + Unpin
        + 'static,
        super::Error,
    >{
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut aggregate: Option<
                objectiveai::vector::completions::response::streaming::VectorCompletionChunk,
            > = None;
            let stream = match self
                .clone()
                .create_streaming(ctx.clone(), request.clone())
                .await
            {
                Ok(stream) => stream,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            futures::pin_mut!(stream);
            while let Some(chunk) = stream.next().await {
                match &mut aggregate {
                    Some(aggregate) => aggregate.push(&chunk),
                    None => aggregate = Some(chunk.clone()),
                }
                let _ = tx.send(Ok(chunk));
            }
            drop(stream);
            drop(tx);
            let response: objectiveai::vector::completions::response::unary::VectorCompletion =
                aggregate.unwrap().into();
            let all_retry_or_cached_or_rng = request
                .retry
                .as_deref()
                .is_some_and(|id| id == response.id.as_str())
                || response.id.is_empty();
            let any_ok_completions =
                response.completions.iter().any(|c| c.error.is_none());
            if any_ok_completions && !all_retry_or_cached_or_rng {
                self.usage_handler
                    .handle_usage(ctx, request, response)
                    .await;
            }
        });
        let mut stream =
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
        match stream.next().await {
            Some(Ok(chunk)) => {
                Ok(StreamOnce::new(chunk).chain(stream.map(Result::unwrap)))
            }
            Some(Err(e)) => Err(e),
            None => unreachable!(),
        }
    }
}

impl<CTXEXT, FENSLLM, CUSG, FENS, FVVOTE, FCVOTE, VUSG>
    Client<CTXEXT, FENSLLM, CUSG, FENS, FVVOTE, FCVOTE, VUSG>
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    FENSLLM: crate::ensemble_llm::fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    CUSG: chat::completions::usage_handler::UsageHandler<CTXEXT>
        + Send
        + Sync
        + 'static,
    FENS: crate::ensemble::fetcher::Fetcher<CTXEXT> + Send + Sync + 'static,
    FVVOTE: super::completion_votes_fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    FCVOTE: super::cache_vote_fetcher::Fetcher<CTXEXT> + Send + Sync + 'static,
    VUSG: Send + Sync + 'static,
{
    /// Creates a streaming vector completion.
    ///
    /// Orchestrates chat completions across all LLMs in the ensemble, extracting
    /// votes from each and combining them with weights to produce scores.
    pub async fn create_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::vector::completions::request::VectorCompletionCreateParams>,
    ) -> Result<
        impl Stream<Item = objectiveai::vector::completions::response::streaming::VectorCompletionChunk>
        + Send
        + 'static,
        super::Error,
    >{
        // timestamp and identify the completion
        let created = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let response_id = response_id(created);

        // validate response count
        let request_responses_len = request.responses.len();
        if request_responses_len < 2 {
            return Err(super::Error::ExpectedTwoOrMoreRequestVectorResponses(
                request_responses_len,
            ));
        }

        // validate credits + fetch ensemble if needed + fetch retry votes if needed
        let (ensemble, mut static_votes) = match (
            &request.ensemble,
            &request.retry,
        ) {
            (
                objectiveai::vector::completions::request::Ensemble::Id(
                    ensemble_id,
                ),
                Some(retry),
            ) => {
                let (ensemble, mut votes) = tokio::try_join!(
                    self.ensemble_fetcher.fetch(ctx.clone(), ensemble_id).map(
                        |result| {
                            match result {
                                Ok(Some((ensemble, _))) => Ok(ensemble),
                                Ok(None) => Err(super::Error::EnsembleNotFound),
                                Err(e) => Err(super::Error::FetchEnsemble(e)),
                            }
                        }
                    ),
                    self.completion_votes_fetcher
                        .fetch(ctx.clone(), retry)
                        .map(|result| {
                            match result {
                                Ok(Some(votes)) => Ok(votes),
                                Ok(None) => Err(super::Error::RetryNotFound),
                                Err(e) => Err(super::Error::FetchRetry(e)),
                            }
                        }),
                )?;
                votes.iter_mut().for_each(|vote| {
                    vote.retry = Some(true);
                    vote.from_cache = Some(true);
                    vote.from_rng = None;
                    vote.completion_index = None;
                });
                (ensemble, votes)
            }
            (
                objectiveai::vector::completions::request::Ensemble::Provided(
                    ensemble_base,
                ),
                Some(retry),
            ) => {
                let ensemble = ensemble_base
                    .clone()
                    .try_into()
                    .map_err(super::Error::InvalidEnsemble)?;
                let mut votes = self
                    .completion_votes_fetcher
                    .fetch(ctx.clone(), retry)
                    .map(|result| match result {
                        Ok(Some(votes)) => Ok(votes),
                        Ok(None) => Err(super::Error::RetryNotFound),
                        Err(e) => Err(super::Error::FetchRetry(e)),
                    })
                    .await?;
                votes.iter_mut().for_each(|vote| {
                    vote.retry = Some(true);
                    vote.from_cache = Some(true);
                    vote.from_rng = None;
                    vote.completion_index = None;
                });
                (ensemble, votes)
            }
            (
                objectiveai::vector::completions::request::Ensemble::Id(
                    ensemble_id,
                ),
                None,
            ) => {
                let ensemble = self
                    .ensemble_fetcher
                    .fetch(ctx.clone(), ensemble_id)
                    .map(|result| match result {
                        Ok(Some((ensemble, _))) => Ok(ensemble),
                        Ok(None) => Err(super::Error::EnsembleNotFound),
                        Err(e) => Err(super::Error::FetchEnsemble(e)),
                    })
                    .await?;
                (ensemble, Vec::new())
            }
            (
                objectiveai::vector::completions::request::Ensemble::Provided(
                    ensemble_base,
                ),
                None,
            ) => {
                let ensemble = ensemble_base
                    .clone()
                    .try_into()
                    .map_err(super::Error::InvalidEnsemble)?;
                (ensemble, Vec::new())
            }
        };

        // prune votes that don't match responses length
        static_votes.retain(|vote| vote.vote.len() == request_responses_len);

        // validate profile
        if request.profile.len() != ensemble.llms.len() {
            return Err(super::Error::InvalidProfile(
                "profile length must match ensemble length".to_string(),
            ));
        }
        let mut positive_weight_count = 0;
        for weight in &request.profile {
            if *weight > Decimal::ZERO {
                if *weight > Decimal::ONE || *weight < Decimal::ZERO {
                    return Err(super::Error::InvalidProfile(
                        "profile weights must be between 0 and 1".to_string(),
                    ));
                } else if *weight > Decimal::ZERO {
                    positive_weight_count += 1;
                }
            }
        }
        if positive_weight_count < 2 {
            return Err(super::Error::InvalidProfile(
                "profile must have two or more positive weights".to_string(),
            ));
        }

        // compute hash IDs
        let prompt_id = {
            let mut prompt = request.messages.clone();
            objectiveai::chat::completions::request::prompt::prepare(
                &mut prompt,
            );
            objectiveai::chat::completions::request::prompt::id(&prompt)
        };
        let tools_id = match &request.tools {
            Some(tools) if !tools.is_empty() => {
                Some(objectiveai::chat::completions::request::tools::id(tools))
            }
            _ => None,
        };
        let responses_ids = {
            let mut responses = request.responses.clone();
            let mut responses_ids = Vec::with_capacity(responses.len());
            for response in &mut responses {
                response.prepare();
                responses_ids.push(response.id());
            }
            responses_ids
        };

        // create a vector of LLMs with useful info
        // only ones that may stream
        let mut llms = ensemble
            .llms
            .into_iter()
            .enumerate()
            .flat_map(|(ensemble_index, llm)| {
                let count = llm.count as usize;
                std::iter::repeat_n(
                    (ensemble_index, llm, request.profile[ensemble_index]),
                    count,
                )
            })
            .enumerate()
            .filter_map(
                |(flat_ensemble_index, (ensemble_index, llm, weight))| {
                    if weight <= Decimal::ZERO {
                        // skip LLMs with zero weight
                        None
                    } else if static_votes.iter().any(|v| {
                        v.flat_ensemble_index == flat_ensemble_index as u64
                    }) {
                        // skip LLMs that have votes already
                        None
                    } else {
                        Some((flat_ensemble_index, ensemble_index, llm, weight))
                    }
                },
            )
            .collect::<Vec<_>>();

        // fetch from cache if requested
        if request.from_cache.is_some_and(|bool| bool) {
            // collect model refs so they're owned here
            let mut model_refs = Vec::with_capacity(llms.len());
            for (_, _, llm, _) in &llms {
                let model =
                    objectiveai::chat::completions::request::Model::Provided(
                        llm.inner.base.clone(),
                    );
                let models = llm.fallbacks.as_ref().map(|fallbacks| {
                    fallbacks
                        .iter()
                        .map(|fallback| objectiveai::chat::completions::request::Model::Provided(
                            fallback.base.clone(),
                        ))
                        .collect::<Vec<_>>()
                });
                model_refs.push((model, models));
            }
            // execute the futures
            let mut futs = Vec::with_capacity(llms.len());
            for (
                (flat_ensemble_index, ensemble_index, _, weight),
                (model, models),
            ) in llms.iter().zip(model_refs.iter())
            {
                let cache_vote_fetcher = self.cache_vote_fetcher.clone();
                let request = request.clone();
                let ctx = ctx.clone();
                let responses_ids = responses_ids.clone();
                futs.push(async move {
                    match cache_vote_fetcher.fetch(
                        ctx,
                        model,
                        models.as_deref(),
                        &request.messages,
                        request.tools.as_deref(),
                        &request.responses,
                    ).await {
                        Ok(Some(mut vote)) => {
                            // update fields
                            vote.ensemble_index = *ensemble_index as u64;
                            vote.flat_ensemble_index = *flat_ensemble_index as u64;
                            vote.weight = *weight;
                            vote.retry = None;
                            vote.from_cache = Some(true);
                            vote.completion_index = None;

                            // rearrange vote vector to match response order
                            let mut rearranged_vote = vec![
                                Decimal::ZERO;
                                request_responses_len
                            ];
                            for (i, response_id) in
                                responses_ids.iter().enumerate()
                            {
                                let pos = vote
                                    .responses_ids
                                    .iter()
                                    .position(|id| id == response_id)
                                    .expect(
                                        "data integrity error: response ID not found in vote responses IDs",
                                    );
                                rearranged_vote[i] = vote.vote[pos];
                            }
                            vote.vote = rearranged_vote;
                            vote.responses_ids = responses_ids;

                            // return vote
                            Ok(Some(vote))
                        }
                        Ok(None) => Ok(None),
                        Err(e) => Err(super::Error::FetchCacheVote(e))
                    }
                });
            }
            let cached_votes = futures::future::try_join_all(futs).await?;
            static_votes.reserve(cached_votes.iter().flatten().count());
            for vote in cached_votes.into_iter().flatten() {
                static_votes.push(vote);
            }
        }

        // filter LLMs that now have votes from cache
        llms.retain(|(flat_ensemble_index, _, _, _)| {
            !static_votes
                .iter()
                .any(|v| v.flat_ensemble_index == *flat_ensemble_index as u64)
        });

        // generate votes with RNG if requested
        if request.from_rng.is_some_and(|bool| bool) {
            let mut rng = rand::rng();
            for (flat_ensemble_index, ensemble_index, llm, weight) in &llms {
                // initialize the vote vector
                let mut vote = vec![Decimal::ZERO; request_responses_len];
                // generate a random value for each entry
                let mut sum = Decimal::ZERO;
                for i in 0..request_responses_len {
                    let v = Decimal::from(rng.random_range(0..=u64::MAX))
                        / Decimal::from(u64::MAX);
                    vote[i] = v;
                    sum += v;
                }
                // normalize the vote vector
                for v in &mut vote {
                    *v /= sum;
                }
                // push the vote
                static_votes.push(
                    objectiveai::vector::completions::response::Vote {
                        model: llm.inner.id.clone(),
                        ensemble_index: *ensemble_index as u64,
                        flat_ensemble_index: *flat_ensemble_index as u64,
                        prompt_id: prompt_id.clone(),
                        tools_id: tools_id.clone(),
                        responses_ids: responses_ids.clone(),
                        vote,
                        weight: *weight,
                        retry: None,
                        from_cache: None,
                        from_rng: Some(true),
                        completion_index: None,
                    },
                );
            }
        }

        // filter LLMs that now have votes from RNG
        llms.retain(|(flat_ensemble_index, _, _, _)| {
            !static_votes
                .iter()
                .any(|v| v.flat_ensemble_index == *flat_ensemble_index as u64)
        });

        // sort retry/cached/rng votes
        static_votes.sort_by_key(|vote| vote.flat_ensemble_index);

        // track usage
        let mut usage =
            objectiveai::vector::completions::response::Usage::default();

        // track scores and weights
        let mut weights = vec![Decimal::ZERO; request_responses_len];
        let mut scores = vec![
            Decimal::ONE
                / Decimal::from(request_responses_len);
            request_responses_len
        ];

        // completion chunk indices are first come first served
        let indexer = Arc::new(ChoiceIndexer::new(0));

        // stream votes from each LLM in the ensemble
        let mut vote_stream =
            futures::stream::select_all(llms.into_iter().map(
                |(flat_ensemble_index, ensemble_index, llm, weight)| {
                    futures::stream::once(self.clone().llm_create_streaming(
                        ctx.clone(),
                        response_id.clone(),
                        created,
                        ensemble.id.clone(),
                        indexer.clone(),
                        llm,
                        ensemble_index,
                        flat_ensemble_index,
                        weight,
                        request.clone(),
                        prompt_id.clone(),
                        tools_id.clone(),
                        responses_ids.clone(),
                    ))
                    .flatten()
                    .boxed()
                },
            ));

        // validate there is at least one retried vote
        if vote_stream.len() == 0 {
            if static_votes.len() > 0 {
                // update weights
                for vote in &static_votes {
                    for (i, v) in vote.vote.iter().enumerate() {
                        weights[i] += *v * vote.weight;
                    }
                }
                // update scores
                let weight_sum: Decimal = weights.iter().sum();
                if weight_sum > Decimal::ZERO {
                    for (i, score) in scores.iter_mut().enumerate() {
                        *score = weights[i] / weight_sum;
                    }
                }
                // return stream of existing votes
                return Ok(futures::future::Either::Left(StreamOnce::new(
                    objectiveai::vector::completions::response::streaming::VectorCompletionChunk {
                        id: request.retry.clone().unwrap_or_default(),
                        completions: Vec::new(),
                        votes: static_votes,
                        scores,
                        weights,
                        created,
                        ensemble: ensemble.id,
                        object: objectiveai::vector::completions::response::streaming::Object::VectorCompletionChunk,
                        usage: None,
                    }
                )));
            } else {
                unreachable!()
            }
        }

        // initial chunk
        let mut next_chunk = match vote_stream.next().await {
            Some(chunk) => Some(chunk),
            None => {
                // should not happen as there should be at least one LLM
                unreachable!()
            }
        };

        Ok(futures::future::Either::Right(async_stream::stream! {
            // stream all chunks
            while let Some(mut chunk) = next_chunk.take() {
                // prepare next chunk
                next_chunk = vote_stream.next().await;

                // if retry votes were provided, add them to the first chunk
                if static_votes.len() > 0 {
                    for vote in chunk.votes.drain(..) {
                        static_votes.push(vote);
                    }
                    chunk.votes = std::mem::take(&mut static_votes);
                }

                // import usage from each completion
                for completion in &chunk.completions
                {
                    if let Some(completion_usage) = &completion.inner.usage {
                        usage.push_chat_completion_usage(&completion_usage);
                    }
                }

                // update weights from votes
                let mut vote_found = false;
                for vote in &chunk.votes {
                    vote_found = true;
                    for (i, v) in vote.vote.iter().enumerate() {
                        weights[i] += *v * vote.weight;
                    }
                }

                // update scores if votes were found
                if vote_found {
                    let weight_sum: Decimal = weights.iter().sum();
                    if weight_sum > Decimal::ZERO {
                        for (i, score) in scores.iter_mut().enumerate() {
                            *score = weights[i] / weight_sum;
                        }
                    }
                }

                // add weights and scores to chunk
                chunk.weights = weights.clone();
                chunk.scores = scores.clone();

                // if on last chunk, add usage
                if next_chunk.is_none() {
                    chunk.usage = Some(usage.clone());
                }

                yield chunk;
            }
        }))
    }

    /// Creates a streaming completion for a single LLM in the ensemble.
    ///
    /// Generates prefix data for vote extraction, streams the chat completion,
    /// and extracts votes from the LLM's response.
    async fn llm_create_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        id: String,
        created: u64,
        ensemble: String,
        indexer: Arc<ChoiceIndexer>,
        llm: objectiveai::ensemble_llm::EnsembleLlmWithFallbacksAndCount,
        ensemble_index: usize,
        flat_ensemble_index: usize,
        weight: Decimal,
        request: Arc<objectiveai::vector::completions::request::VectorCompletionCreateParams>,
        prompt_id: String,
        tools_id: Option<String>,
        responses_ids: Vec<String>,
    ) -> impl Stream<Item = objectiveai::vector::completions::response::streaming::VectorCompletionChunk> + Send + 'static
    {
        let request_responses_len = request.responses.len();

        // create pfx data for each LLM
        let (vector_pfx_data, vector_pfx_indices) = {
            let mut rng = rand::rng();
            let mut vector_pfx_data = HashMap::with_capacity(
                1 + llm.fallbacks.as_ref().map(Vec::len).unwrap_or(0),
            );
            let mut vector_pfx_indices = Vec::with_capacity(
                1 + llm.fallbacks.as_ref().map(Vec::len).unwrap_or(0),
            );
            for llm in std::iter::once(&llm.inner).chain(
                llm.fallbacks
                    .iter()
                    .map(|fallbacks| fallbacks.iter())
                    .flatten(),
            ) {
                // create the prefixes
                let pfx_tree = super::PfxTree::new(
                    &mut rng,
                    request_responses_len,
                    match llm.base.top_logprobs {
                        Some(0) | Some(1) | None => 20,
                        Some(top_logprobs) => top_logprobs as usize,
                    },
                );

                // map prefix to response index
                let pfx_indices =
                    pfx_tree.pfx_indices(&mut rng, request_responses_len);

                let (
                    // regex capture pattern matching response keys as-is
                    responses_key_pattern,
                    // regex capture pattern matching response keys stripped of first and last tick
                    responses_key_pattern_stripped,
                ) = pfx_tree.regex_patterns(&pfx_indices);

                vector_pfx_data.insert(
                    llm.id.clone(),
                    super::PfxData {
                        pfx_tree,
                        responses_key_pattern,
                        responses_key_pattern_stripped,
                    },
                );
                vector_pfx_indices.push(Arc::new(pfx_indices));
            }
            (vector_pfx_data, vector_pfx_indices)
        };

        // stream
        let mut stream = match self
            .chat_client
            .clone()
            .create_streaming_for_vector_handle_usage(
                ctx,
                request,
                vector_pfx_indices,
                llm,
            )
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                return futures::future::Either::Left(
                    Self::llm_create_streaming_vector_error(
                        id,
                        indexer.get(flat_ensemble_index),
                        e,
                        created,
                        ensemble,
                    ),
                );
            }
        };

        // only return error if the very first stream item is an error
        let mut next_chat_chunk = match stream.try_next().await {
            Ok(Some(chunk)) => Some(chunk),
            Err(e) => {
                return futures::future::Either::Left(
                    Self::llm_create_streaming_vector_error(
                        id,
                        indexer.get(flat_ensemble_index),
                        e,
                        created,
                        ensemble,
                    ),
                );
            }
            Ok(None) => {
                // chat client will always yield at least 1 item
                unreachable!()
            }
        };

        // the aggregate of all chunks
        let mut aggregate: Option<
            objectiveai::vector::completions::response::streaming::VectorCompletionChunk,
        > = None;

        futures::future::Either::Right(async_stream::stream! {
            while let Some(chat_chunk) = next_chat_chunk.take() {
                // fetch the next chat chunk or error
                let error = match stream.next().await {
                    Some(Ok(ncc)) => {
                        // set next chat chunk
                        next_chat_chunk = Some(ncc);
                        None
                    }
                    Some(Err(e)) => {
                        // end the loop after this iteration
                        // add error to choices
                        Some(objectiveai::error::ResponseError::from(&e))
                    }
                    None => {
                        // end the loop after this iteration
                        None
                    }
                };

                // construct the vector completions chunk from the chat completions chunk
                let mut chunk = objectiveai::vector::completions::response::streaming::VectorCompletionChunk {
                    id: id.clone(),
                    completions: vec![
                        objectiveai::vector::completions::response::streaming::ChatCompletionChunk {
                            index: indexer.get(flat_ensemble_index),
                            inner: chat_chunk,
                            error,
                        },
                    ],
                    votes: Vec::new(),
                    scores: Vec::new(),
                    weights: Vec::new(),
                    created,
                    ensemble: ensemble.clone(),
                    object: objectiveai::vector::completions::response::streaming::Object::VectorCompletionChunk,
                    usage: None,
                };

                // push the chunk into the aggregate
                match aggregate {
                    Some(ref mut aggregate) => {
                        aggregate.push(&chunk);
                    }
                    None => {
                        aggregate = Some(chunk.clone());
                    }
                }

                // if last chunk, add votes
                if next_chat_chunk.is_none() {
                    let aggregate = aggregate.take().unwrap();
                    for completion in aggregate.completions {
                        // get pfx data for this LLM
                        let super::PfxData {
                            pfx_tree,
                            responses_key_pattern,
                            responses_key_pattern_stripped,
                        } = &vector_pfx_data[&completion.inner.model];

                        // try to get votes for each choice
                        for choice in completion.inner.choices {
                            if let Some(vote) = super::get_vote(
                                pfx_tree.clone(),
                                &responses_key_pattern,
                                &responses_key_pattern_stripped,
                                request_responses_len,
                                &choice,
                            ) {
                                chunk.votes.push(objectiveai::vector::completions::response::Vote {
                                    model: completion.inner.model.clone(),
                                    ensemble_index: ensemble_index as u64,
                                    flat_ensemble_index: flat_ensemble_index as u64,
                                    prompt_id: prompt_id.clone(),
                                    tools_id: tools_id.clone(),
                                    responses_ids: responses_ids.clone(),
                                    vote,
                                    weight,
                                    retry: None,
                                    from_cache: None,
                                    from_rng: None,
                                    completion_index: Some(completion.index),
                                });
                            }
                        }
                    }
                }

                // yield chunk
                yield chunk;
            }
        })
    }

    /// Creates an error response chunk for a failed LLM completion.
    fn llm_create_streaming_vector_error(
        id: String,
        completion_index: u64,
        error: chat::completions::Error,
        created: u64,
        ensemble: String,
    ) -> impl Stream<Item = objectiveai::vector::completions::response::streaming::VectorCompletionChunk>
    + Send
    + Unpin
    + 'static
    {
        StreamOnce::new(
            objectiveai::vector::completions::response::streaming::VectorCompletionChunk {
                id,
                completions: vec![
                    objectiveai::vector::completions::response::streaming::ChatCompletionChunk {
                        index: completion_index,
                        inner: objectiveai::chat::completions::response::streaming::ChatCompletionChunk::default(),
                        error: Some(objectiveai::error::ResponseError::from(&error)),
                    },
                ],
                votes: Vec::new(),
                scores: Vec::new(),
                weights: Vec::new(),
                created,
                ensemble,
                object: objectiveai::vector::completions::response::streaming::Object::VectorCompletionChunk,
                usage: None,
            }
        )
    }
}
