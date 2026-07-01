//! Vector completion client implementation.

use crate::{
    agent, ctx,
    util::{ChoiceIndexer, StreamOnce},
};
use futures::{FutureExt, Stream, StreamExt, TryStreamExt};
use rand::{Rng, SeedableRng};
use rust_decimal::Decimal;
use std::{collections::HashMap, hash::Hasher, sync::Arc, time};

/// Generates a unique response ID for a vector completion.
pub fn response_id(created: u64) -> String {
    crate::util::response_id(Some("vctcpl"), created)
}

pub(super) fn invert_and_l1_normalize(mut xs: Vec<Decimal>) -> Vec<Decimal> {
    if xs.is_empty() {
        return xs;
    }
    for x in &mut xs {
        *x = Decimal::ONE - *x;
    }
    let sum: Decimal = xs.iter().map(|x| x.abs()).sum();
    if sum == Decimal::ZERO {
        let uniform = Decimal::ONE / Decimal::from(xs.len());
        for x in &mut xs {
            *x = uniform;
        }
    } else {
        for x in &mut xs {
            *x /= sum;
        }
    }
    xs
}

#[cfg(test)]
mod invert_and_l1_normalize_tests {
    use super::invert_and_l1_normalize;
    use rust_decimal::dec;

    #[test]
    fn example() {
        let v = vec![dec!(0.75), dec!(0.25), dec!(0.0)];
        let out = invert_and_l1_normalize(v);
        assert_eq!(out, vec![dec!(0.125), dec!(0.375), dec!(0.5)]);
    }

    #[test]
    fn uniform_when_all_ones() {
        let v = vec![dec!(1.0), dec!(1.0), dec!(1.0), dec!(1.0)];
        let out = invert_and_l1_normalize(v);
        assert_eq!(out, vec![dec!(0.25), dec!(0.25), dec!(0.25), dec!(0.25)]);
    }
}
/// Client for creating vector completions.
///
/// Orchestrates multiple LLM agent completions to vote on response options,
/// combining their votes using weights to produce final scores.
pub struct Client<
    CTXEXT,
    OPENROUTER,
    CLAUDEAGENTSDK,
    CODEXSDK,
    MOCK,
    RETRG,
    RETRF,
    RETRM,
    ACUSG,
    VUSG,
> {
    /// The underlying agent completion client.
    pub agent_client: Arc<
        agent::completions::Client<
            CTXEXT,
            OPENROUTER,
            CLAUDEAGENTSDK,
            CODEXSDK,
            MOCK,
            RETRG,
            RETRF,
            RETRM,
            ACUSG,
        >,
    >,
    /// Retrieve router for resolving swarms and agents.
    pub retrieve_router:
        Arc<crate::retrieval::retrieve::Router<RETRG, RETRF, RETRM, CTXEXT>>,
    /// Handler for usage tracking.
    pub usage_handler: Arc<VUSG>,
}

impl<
    CTXEXT,
    OPENROUTER,
    CLAUDEAGENTSDK,
    CODEXSDK,
    MOCK,
    RETRG,
    RETRF,
    RETRM,
    ACUSG,
    VUSG,
>
    Client<
        CTXEXT,
        OPENROUTER,
        CLAUDEAGENTSDK,
        CODEXSDK,
        MOCK,
        RETRG,
        RETRF,
        RETRM,
        ACUSG,
        VUSG,
    >
{
    /// Creates a new vector completion client.
    pub fn new(
        agent_client: Arc<
            agent::completions::Client<
                CTXEXT,
                OPENROUTER,
                CLAUDEAGENTSDK,
                CODEXSDK,
                MOCK,
                RETRG,
                RETRF,
                RETRM,
                ACUSG,
            >,
        >,
        retrieve_router: Arc<
            crate::retrieval::retrieve::Router<RETRG, RETRF, RETRM, CTXEXT>,
        >,
        usage_handler: Arc<VUSG>,
    ) -> Self {
        Self {
            agent_client,
            retrieve_router,
            usage_handler,
        }
    }
}

impl<
    CTXEXT,
    OPENROUTER,
    CLAUDEAGENTSDK,
    CODEXSDK,
    MOCK,
    RETRG,
    RETRF,
    RETRM,
    ACUSG,
    VUSG,
>
    Client<
        CTXEXT,
        OPENROUTER,
        CLAUDEAGENTSDK,
        CODEXSDK,
        MOCK,
        RETRG,
        RETRF,
        RETRM,
        ACUSG,
        VUSG,
    >
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    OPENROUTER: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::openrouter::Agent,
            objectiveai_sdk::agent::openrouter::Continuation,
        > + Send
        + Sync
        + 'static,
    CLAUDEAGENTSDK: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent,
            objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    CODEXSDK: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent,
            objectiveai_sdk::agent::codex_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    MOCK: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::mock::Agent,
            objectiveai_sdk::agent::mock::Continuation,
        > + Send
        + Sync
        + 'static,
    RETRG: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRF: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRM: crate::retrieval::retrieve::Client<CTXEXT>,
    ACUSG: agent::completions::usage_handler::UsageHandler<CTXEXT>
        + Send
        + Sync
        + 'static,
    VUSG: super::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
{
    /// Creates a unary (non-streaming) vector completion with usage tracking.
    ///
    /// Collects all streaming chunks into a single response.
    pub async fn create_unary_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams>,
    ) -> Result<
        objectiveai_sdk::vector::completions::response::unary::VectorCompletion,
        super::Error,
    > {
        let mut aggregate: Option<
            objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk,
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
        request: Arc<objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams>,
    ) -> Result<
        impl Stream<Item = objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk>
        + Send
        + Unpin
        + 'static,
        super::Error,
    >{
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut aggregate: Option<
                objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk,
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
                if tx.send(Ok(chunk)).is_err() {
                    ctx.cancel();
                }
            }
            drop(stream);
            drop(tx);
            let response: objectiveai_sdk::vector::completions::response::unary::VectorCompletion =
                aggregate.unwrap().into();
            if response.usage.any_usage() {
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

impl<
    CTXEXT,
    OPENROUTER,
    CLAUDEAGENTSDK,
    CODEXSDK,
    MOCK,
    RETRG,
    RETRF,
    RETRM,
    ACUSG,
    VUSG,
>
    Client<
        CTXEXT,
        OPENROUTER,
        CLAUDEAGENTSDK,
        CODEXSDK,
        MOCK,
        RETRG,
        RETRF,
        RETRM,
        ACUSG,
        VUSG,
    >
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    OPENROUTER: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::openrouter::Agent,
            objectiveai_sdk::agent::openrouter::Continuation,
        > + Send
        + Sync
        + 'static,
    CLAUDEAGENTSDK: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent,
            objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    CODEXSDK: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent,
            objectiveai_sdk::agent::codex_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    MOCK: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::mock::Agent,
            objectiveai_sdk::agent::mock::Continuation,
        > + Send
        + Sync
        + 'static,
    RETRG: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRF: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRM: crate::retrieval::retrieve::Client<CTXEXT>,
    ACUSG: agent::completions::usage_handler::UsageHandler<CTXEXT>
        + Send
        + Sync
        + 'static,
    VUSG: Send + Sync + 'static,
{
    /// Creates a streaming vector completion.
    ///
    /// Orchestrates agent completions across all LLMs in the swarm, extracting
    /// votes from each and combining them with weights to produce scores.
    pub async fn create_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams>,
    ) -> Result<
        impl Stream<Item = objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk>
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

        // resolve and convert swarm via retrieve router
        let swarm = self
            .retrieve_router
            .get_swarm(&ctx, request.swarm.clone())
            .await
            .map_err(|e| super::Error::InvalidSwarm(e.message.to_string()))?
            .into_inline();

        // extract profile weights from swarm (already validated during conversion)
        let agent_count = swarm.agents.len();
        if agent_count == 0 {
            return Err(super::Error::InvalidSwarm(
                "swarm must have at least one agent".to_string(),
            ));
        }
        let profile_pairs: Vec<(Decimal, bool)> =
            swarm.weights.to_weights_and_invert();

        // compute hash IDs
        let prompt_id = {
            let mut prompt = request.messages.clone();
            objectiveai_sdk::agent::completions::message::prompt::prepare(
                &mut prompt,
            );
            objectiveai_sdk::agent::completions::message::prompt::id(&prompt)
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
        let flat_swarm_len =
            swarm.agents.iter().map(|a| a.count as usize).sum::<usize>();
        let llms = swarm
            .agents
            .into_iter()
            .enumerate()
            .flat_map(|(swarm_index, agent)| {
                let count = agent.count as usize;
                let (weight, invert) = profile_pairs[swarm_index];
                std::iter::repeat_n((swarm_index, agent, weight, invert), count)
            })
            .enumerate()
            .filter_map(
                |(flat_swarm_index, (swarm_index, agent, weight, invert))| {
                    if weight <= Decimal::ZERO {
                        // skip agents with zero weight
                        None
                    } else {
                        Some((
                            flat_swarm_index,
                            swarm_index,
                            agent,
                            weight,
                            invert,
                        ))
                    }
                },
            )
            .collect::<Vec<_>>();

        // track usage
        let mut usage =
            objectiveai_sdk::agent::completions::response::Usage::default();

        // track scores and weights
        let mut weights = vec![Decimal::ZERO; request_responses_len];
        let mut scores = vec![
            Decimal::ONE
                / Decimal::from(request_responses_len);
            request_responses_len
        ];

        // completion chunk indices are first come first served
        let indexer = Arc::new(ChoiceIndexer::new(0));

        // stream votes from each LLM in the swarm
        let mut vote_stream =
            futures::stream::select_all(llms.into_iter().map(
                |(flat_swarm_index, swarm_index, agent, weight, invert)| {
                    futures::stream::once(self.clone().llm_create_streaming(
                        ctx.clone(),
                        response_id.clone(),
                        created,
                        swarm.id.clone(),
                        indexer.clone(),
                        agent,
                        swarm_index,
                        flat_swarm_index,
                        flat_swarm_len,
                        weight,
                        invert,
                        request.clone(),
                        prompt_id.clone(),
                        responses_ids.clone(),
                    ))
                    .flatten()
                    .boxed()
                },
            ));

        // require at least one agent with positive weight to vote
        if vote_stream.len() == 0 {
            return Err(super::Error::InvalidSwarm(
                "swarm has no agents with positive weight".to_string(),
            ));
        }

        // initial chunk
        let mut next_chunk = match vote_stream.next().await {
            Some(chunk) => Some(chunk),
            None => {
                // should not happen as there should be at least one LLM
                unreachable!()
            }
        };

        Ok(async_stream::stream! {
            // stream all chunks
            while let Some(mut chunk) = next_chunk.take() {
                // prepare next chunk
                next_chunk = vote_stream.next().await;

                // import usage from each completion
                for completion in &chunk.completions
                {
                    if let Some(completion_usage) = &completion.inner.usage {
                        usage.push(&completion_usage);
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
        })
    }

    /// Creates a completion for a single LLM in the swarm, extracting its vote.
    ///
    /// Builds an AgentCompletionCreateParams with an inline agent for each LLM,
    /// sends the request via the agent completions client using per-agent
    /// transform_messages closures to attach voting instructions, and extracts
    /// votes from the response.
    async fn llm_create_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        id: String,
        created: u64,
        swarm: String,
        indexer: Arc<ChoiceIndexer>,
        agent: objectiveai_sdk::agent::AgentWithFallbacksWithCount,
        swarm_index: usize,
        flat_swarm_index: usize,
        flat_swarm_len: usize,
        weight: Decimal,
        invert_vote: bool,
        request: Arc<objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams>,
        prompt_id: String,
        responses_ids: Vec<String>,
    ) -> impl Stream<Item = objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk> + Send + 'static
    {
        use objectiveai_sdk::agent::completions::message::{
            Message, RichContent, UserMessage,
        };

        let request_responses_len = request.responses.len();

        // create pfx data and pfx indices for each agent (primary + fallbacks)
        let mut vector_pfx_data: HashMap<String, super::PfxData> =
            HashMap::new();
        let mut vector_pfx_indices: HashMap<String, Vec<(String, usize)>> =
            HashMap::new();
        {
            for a in std::iter::once(agent.inner.agent()).chain(
                agent
                    .inner
                    .fallbacks()
                    .into_iter()
                    .flat_map(|fallbacks| fallbacks.iter()),
            ) {
                let agent_instance_hierarchy = a.id().to_string();
                let mut rng = make_rng(request.seed.map(|s| {
                    per_agent_seed(
                        s,
                        &agent_instance_hierarchy,
                        flat_swarm_index,
                        &prompt_id,
                        &responses_ids,
                    ) as u64
                }));
                let top_logprobs = a.top_logprobs();
                let pfx_tree = super::PfxTree::new(
                    &mut rng,
                    request_responses_len,
                    match top_logprobs {
                        Some(0) | Some(1) | None => 20,
                        Some(top_logprobs) => top_logprobs as usize,
                    },
                );
                let pfx_indices =
                    pfx_tree.pfx_indices(&mut rng, request_responses_len);
                let responses_key_pattern =
                    pfx_tree.regex_pattern(&pfx_indices);
                vector_pfx_data.insert(
                    agent_instance_hierarchy.clone(),
                    super::PfxData {
                        pfx_tree,
                        responses_key_pattern,
                        invert_vote,
                    },
                );
                vector_pfx_indices
                    .insert(agent_instance_hierarchy, pfx_indices);
            }
        }

        // Determine the output mode
        let output_mode = agent.inner.base().output_mode();

        // Build per-agent transform_messages closures
        let transform_messages: agent::completions::TransformMessages = {
            let mut map: agent::completions::TransformMessages = HashMap::new();
            for (agent_instance_hierarchy, pfx_indices) in &vector_pfx_indices {
                let responses = request.responses.clone();
                let pfx_indices = pfx_indices.clone();
                let output_mode = output_mode;

                map.insert(
                    agent_instance_hierarchy.clone(),
                    Box::new(move |messages: Vec<Message>| -> Vec<Message> {
                        transform_messages_for_vector(
                            messages,
                            &responses,
                            &pfx_indices,
                            output_mode,
                        )
                    }),
                );
            }
            map
        };

        // Extract synthetic_reasoning from the primary agent.
        // Only Openrouter supports synthetic_reasoning (requires ToolCall output
        // mode); Claude Agent SDK, Claude Code, and Mock only support Instruction.
        let synthetic_reasoning = match agent.inner.agent() {
            objectiveai_sdk::agent::InlineAgent::Openrouter(a) => {
                a.base.synthetic_reasoning.unwrap_or(false)
            }
            objectiveai_sdk::agent::InlineAgent::ClaudeAgentSdk(_) => false,
            objectiveai_sdk::agent::InlineAgent::CodexSdk(_) => false,
            objectiveai_sdk::agent::InlineAgent::Mock(_) => false,
        };

        // Build per-agent response formats for json_schema and tool_call modes
        let response_format = match output_mode {
            objectiveai_sdk::agent::OutputMode::JsonSchema => {
                let mut per_agent = indexmap::IndexMap::new();
                for (agent_instance_hierarchy, pfx_indices) in
                    &vector_pfx_indices
                {
                    let keys: Vec<String> =
                        pfx_indices.iter().map(|(k, _)| k.clone()).collect();
                    per_agent.insert(
                        agent_instance_hierarchy.clone(),
                        super::ResponseKey::response_format(
                            keys,
                            synthetic_reasoning,
                        ),
                    );
                }
                Some(objectiveai_sdk::agent::completions::request::ResponseFormatParam::PerAgent(per_agent))
            }
            objectiveai_sdk::agent::OutputMode::ToolCall => {
                let mut per_agent = indexmap::IndexMap::new();
                for (agent_instance_hierarchy, pfx_indices) in
                    &vector_pfx_indices
                {
                    let keys: Vec<String> =
                        pfx_indices.iter().map(|(k, _)| k.clone()).collect();
                    per_agent.insert(
                        agent_instance_hierarchy.clone(),
                        super::ResponseKey::tool(keys, synthetic_reasoning),
                    );
                }
                Some(objectiveai_sdk::agent::completions::request::ResponseFormatParam::PerAgent(per_agent))
            }
            objectiveai_sdk::agent::OutputMode::Instruction => None,
        };

        let primary_id = agent.inner.id().to_string();

        // Build the AgentCompletionCreateParams (messages are NOT modified here)
        let inline_wf = agent.inner.inline();
        let agent_params = Arc::new(objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
            messages: request.messages.clone(),
            provider: request.provider.clone(),
            agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
                objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                    inner: inline_wf.inner.clone().into_base(),
                    fallbacks: inline_wf.fallbacks.as_ref().map(|fbs| {
                        fbs.iter().map(|fb| fb.clone().into_base()).collect()
                    }),
                },
            ),
            response_format: response_format.clone(),
            seed: request.seed.map(|s| per_agent_seed(s, &primary_id, flat_swarm_index, &prompt_id, &responses_ids)),
            stream: Some(false),
            continuation: request.continuation.clone(),
            laboratories: None,
        });

        // Call the agent completions client, yielding each chunk immediately
        let transform_messages = Arc::new(transform_messages);

        // Helper to wrap an agent chunk into a VectorCompletionChunk
        let wrap_agent_chunk = {
            let id = id.clone();
            let swarm = swarm.clone();
            move |completion_index: u64, inner: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk| {
                objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk {
                    id: id.clone(),
                    completions: vec![
                        objectiveai_sdk::vector::completions::response::streaming::AgentCompletionChunk {
                            index: completion_index,
                            inner,
                        },
                    ],
                    votes: Vec::new(),
                    scores: Vec::new(),
                    weights: Vec::new(),
                    created,
                    swarm: swarm.clone(),
                    object: objectiveai_sdk::vector::completions::response::streaming::Object::VectorCompletionChunk,
                    usage: None,
                }
            }
        };

        async_stream::stream! {
            // Stream the first call, yielding each chunk immediately while also aggregating
            let first_result = async {
                let stream = self.agent_client.clone().create_streaming_handle_usage(
                    ctx.clone(),
                    agent_params.clone(),
                    None,
                    None, // disable_tools
                    vec![],
                    indexmap::IndexMap::new(),
                    Some(transform_messages.clone()),
                ).await?;
                let aggregate: Option<
                    objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
                > = None;
                let continuation = None;
                Ok::<_, agent::completions::Error>((stream, aggregate, continuation))
            }.await;

            let (mut stream, mut aggregate, mut continuation) = match first_result {
                Ok((stream, aggregate, continuation)) => (stream, aggregate, continuation),
                Err(e) => {
                    yield Self::llm_create_streaming_vector_error(
                        id.clone(), indexer.get(flat_swarm_index), e, agent.inner.base().upstream(), created, swarm.clone(),
                    );
                    return;
                }
            };

            while let Some(item) = stream.next().await {
                match item {
                    agent::completions::StreamItem::Chunk(chunk) => {
                        // Yield immediately
                        yield wrap_agent_chunk(indexer.get(flat_swarm_index), chunk.clone());
                        // Also aggregate for vote extraction
                        match &mut aggregate {
                            Some(agg) => agg.push(&chunk),
                            None => aggregate = Some(chunk),
                        }
                    }
                    agent::completions::StreamItem::State(cont) => {
                        continuation = Some(cont);
                    }
                }
            }
            drop(stream);

            // Convert aggregate to unary for vote extraction
            let response: objectiveai_sdk::agent::completions::response::unary::AgentCompletion =
                match aggregate {
                    Some(agg) => agg.into(),
                    None => return,
                };

            // Extract text and logprobs from the last assistant message
            let (text, logprobs, tool_call_text) = extract_assistant_content(&response);

            // Determine which text to use for vote extraction based on output mode
            let vote_text = match output_mode {
                objectiveai_sdk::agent::OutputMode::ToolCall => {
                    tool_call_text.as_deref().unwrap_or(text.as_deref().unwrap_or(""))
                }
                _ => {
                    text.as_deref().unwrap_or("")
                }
            };

            // Identifiers off the slot's outer completion. The
            // hierarchy is the routing key (matches the
            // `vector_pfx_data` / `vector_pfx_indices` maps); the
            // full-id / leaf-id pair is what we stamp on every Vote
            // (deterministic across api processes, unlike the
            // hierarchy's per-process suffix).
            let agent_instance_hierarchy = response.agent_instance_hierarchy.clone();
            let agent_full_id = response.agent_full_id.clone();
            let agent_id = response.agent_id.clone();
            drop(response);

            // Look up pfx data for the agent ID
            let pfx_data = vector_pfx_data.get(&agent_instance_hierarchy)
                .or_else(|| vector_pfx_data.get(&primary_id));

            let mut votes = Vec::new();

            if let Some(pfx_data) = pfx_data {
                let (match_count, vote) = super::get_vote(
                    pfx_data.pfx_tree.clone(),
                    &pfx_data.responses_key_pattern,
                    request_responses_len,
                    vote_text,
                    logprobs.as_ref(),
                );

                match output_mode {
                    objectiveai_sdk::agent::OutputMode::Instruction => {
                        if match_count == 1 {
                            let vote = if invert_vote {
                                invert_and_l1_normalize(vote)
                            } else {
                                vote
                            };
                            votes.push(objectiveai_sdk::vector::completions::response::Vote {
                                agent_full_id: agent_full_id.clone(),
                                agent_id: agent_id.clone(),
                                swarm_index: swarm_index as u64,
                                flat_swarm_index: flat_swarm_index as u64,
                                prompt_id: prompt_id.clone(),
                                responses_ids: responses_ids.clone(),
                                vote,
                                weight,
                                completion_index: Some(indexer.get(flat_swarm_index)),
                            });
                        } else if let Some(mut cont) = continuation.take() {
                            // Retry via continuation — stream chunks immediately
                            let model_pfx_indices = vector_pfx_indices.get(&agent_instance_hierarchy)
                                .or_else(|| vector_pfx_indices.get(&primary_id))
                                .unwrap();
                            let instruction_suffix = {
                                let mut text = String::from("Output one response key including backticks:\n- ");
                                text.push_str(
                                    &model_pfx_indices.iter()
                                        .map(|(key, _)| key.clone())
                                        .collect::<Vec<_>>()
                                        .join("\n- "),
                                );
                                text
                            };
                            let retry_message = format!(
                                "Your response included {} response keys.\n\n{}",
                                match_count,
                                &instruction_suffix,
                            );

                            cont.push_user_message(
                                UserMessage {
                                    content: RichContent::Text(retry_message),
                                },
                            );

                            match self.agent_client.clone().create_streaming_handle_usage(
                                ctx.clone(),
                                agent_params.clone(),
                                Some(cont),
                                None, // disable_tools
                                vec![],
                                indexmap::IndexMap::new(),
                                Some(transform_messages.clone()),
                            ).await {
                                Ok(mut retry_stream) => {
                                    let mut retry_agg: Option<
                                        objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
                                    > = None;
                                    while let Some(item) = retry_stream.next().await {
                                        match item {
                                            agent::completions::StreamItem::Chunk(chunk) => {
                                                yield wrap_agent_chunk(indexer.get(flat_swarm_index + flat_swarm_len), chunk.clone());
                                                match &mut retry_agg {
                                                    Some(agg) => agg.push(&chunk),
                                                    None => retry_agg = Some(chunk),
                                                }
                                            }
                                            agent::completions::StreamItem::State(_) => {}
                                        }
                                    }
                                    if let Some(retry_agg) = retry_agg {
                                        let retry_response: objectiveai_sdk::agent::completions::response::unary::AgentCompletion = retry_agg.into();
                                        let (retry_text, retry_logprobs, _) = extract_assistant_content(&retry_response);
                                        let retry_vote_text = retry_text.as_deref().unwrap_or("");
                                        let (retry_count, retry_vote) = super::get_vote(
                                            pfx_data.pfx_tree.clone(),
                                            &pfx_data.responses_key_pattern,
                                            request_responses_len,
                                            retry_vote_text,
                                            retry_logprobs.as_ref(),
                                        );
                                        if retry_count > 0 {
                                            let retry_vote = if invert_vote {
                                                invert_and_l1_normalize(retry_vote)
                                            } else {
                                                retry_vote
                                            };
                                            votes.push(objectiveai_sdk::vector::completions::response::Vote {
                                                agent_full_id: agent_full_id.clone(),
                                                agent_id: agent_id.clone(),
                                                swarm_index: swarm_index as u64,
                                                flat_swarm_index: flat_swarm_index as u64,
                                                prompt_id: prompt_id.clone(),
                                                responses_ids: responses_ids.clone(),
                                                vote: retry_vote,
                                                weight,
                                                completion_index: Some(indexer.get(flat_swarm_index + flat_swarm_len)),
                                            });
                                        }
                                    }
                                }
                                Err(_) => {
                                    // Retry failed, use original vote if we had multi-match
                                    if match_count > 1 {
                                        let vote = if invert_vote {
                                            invert_and_l1_normalize(vote)
                                        } else {
                                            vote
                                        };
                                        votes.push(objectiveai_sdk::vector::completions::response::Vote {
                                            agent_full_id: agent_full_id.clone(),
                                            agent_id: agent_id.clone(),
                                            swarm_index: swarm_index as u64,
                                            flat_swarm_index: flat_swarm_index as u64,
                                            prompt_id: prompt_id.clone(),
                                            responses_ids: responses_ids.clone(),
                                            vote,
                                            weight,
                                            completion_index: Some(indexer.get(flat_swarm_index)),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    objectiveai_sdk::agent::OutputMode::ToolCall => {
                        if tool_call_text.is_some() && match_count > 0 {
                            let vote = if invert_vote {
                                invert_and_l1_normalize(vote)
                            } else {
                                vote
                            };
                            votes.push(objectiveai_sdk::vector::completions::response::Vote {
                                agent_full_id: agent_full_id.clone(),
                                agent_id: agent_id.clone(),
                                swarm_index: swarm_index as u64,
                                flat_swarm_index: flat_swarm_index as u64,
                                prompt_id: prompt_id.clone(),
                                responses_ids: responses_ids.clone(),
                                vote,
                                weight,
                                completion_index: Some(indexer.get(flat_swarm_index)),
                            });
                        } else if let Some(mut cont) = continuation.take() {
                            // Retry with required: true — stream chunks immediately
                            let mut retry_rf = indexmap::IndexMap::new();
                            for (agent_instance_hierarchy, pfx_indices) in &vector_pfx_indices {
                                let keys: Vec<String> = pfx_indices.iter().map(|(k, _)| k.clone()).collect();
                                let think = synthetic_reasoning;
                                retry_rf.insert(
                                    agent_instance_hierarchy.clone(),
                                    super::ResponseKey::tool_required(keys, think),
                                );
                            }

                            cont.push_user_message(
                                UserMessage {
                                    content: RichContent::Text(
                                        "Use the response_key tool to select a response.".to_string(),
                                    ),
                                },
                            );

                            let retry_params = Arc::new(objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
                                response_format: Some(objectiveai_sdk::agent::completions::request::ResponseFormatParam::PerAgent(retry_rf)),
                                ..(*agent_params).clone()
                            });

                            match self.agent_client.clone().create_streaming_handle_usage(
                                ctx.clone(),
                                retry_params,
                                Some(cont),
                                None, // disable_tools
                                vec![],
                                indexmap::IndexMap::new(),
                                Some(transform_messages.clone()),
                            ).await {
                                Ok(mut retry_stream) => {
                                    let mut retry_agg: Option<
                                        objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
                                    > = None;
                                    while let Some(item) = retry_stream.next().await {
                                        match item {
                                            agent::completions::StreamItem::Chunk(chunk) => {
                                                yield wrap_agent_chunk(indexer.get(flat_swarm_index + flat_swarm_len), chunk.clone());
                                                match &mut retry_agg {
                                                    Some(agg) => agg.push(&chunk),
                                                    None => retry_agg = Some(chunk),
                                                }
                                            }
                                            agent::completions::StreamItem::State(_) => {}
                                        }
                                    }
                                    if let Some(retry_agg) = retry_agg {
                                        let retry_response: objectiveai_sdk::agent::completions::response::unary::AgentCompletion = retry_agg.into();
                                        let (_, retry_logprobs, retry_tc_text) = extract_assistant_content(&retry_response);
                                        if let Some(tc_text) = retry_tc_text {
                                            let retry_agent_instance_hierarchy = retry_response.agent_instance_hierarchy.clone();
                                            let retry_pfx = vector_pfx_data.get(&retry_agent_instance_hierarchy)
                                                .or_else(|| vector_pfx_data.get(&primary_id));
                                            if let Some(retry_pfx) = retry_pfx {
                                                let (retry_count, retry_vote) = super::get_vote(
                                                    retry_pfx.pfx_tree.clone(),
                                                    &retry_pfx.responses_key_pattern,
                                                    request_responses_len,
                                                    &tc_text,
                                                    retry_logprobs.as_ref(),
                                                );
                                                if retry_count > 0 {
                                                    let retry_vote = if invert_vote {
                                                        invert_and_l1_normalize(retry_vote)
                                                    } else {
                                                        retry_vote
                                                    };
                                                    votes.push(objectiveai_sdk::vector::completions::response::Vote {
                                                        agent_full_id: agent_full_id.clone(),
                                                        agent_id: agent_id.clone(),
                                                        swarm_index: swarm_index as u64,
                                                        flat_swarm_index: flat_swarm_index as u64,
                                                        prompt_id: prompt_id.clone(),
                                                        responses_ids: responses_ids.clone(),
                                                        vote: retry_vote,
                                                        weight,
                                                        completion_index: Some(indexer.get(flat_swarm_index + flat_swarm_len)),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(_) => {
                                    // No vote for this LLM
                                }
                            }
                        }
                    }
                    objectiveai_sdk::agent::OutputMode::JsonSchema => {
                        if match_count > 0 {
                            let vote = if invert_vote {
                                invert_and_l1_normalize(vote)
                            } else {
                                vote
                            };
                            votes.push(objectiveai_sdk::vector::completions::response::Vote {
                                agent_full_id: agent_full_id.clone(),
                                agent_id: agent_id.clone(),
                                swarm_index: swarm_index as u64,
                                flat_swarm_index: flat_swarm_index as u64,
                                prompt_id: prompt_id.clone(),
                                responses_ids: responses_ids.clone(),
                                vote,
                                weight,
                                completion_index: Some(indexer.get(flat_swarm_index)),
                            });
                        }
                    }
                }
            }

            // Yield a final chunk with just the votes (completions already yielded)
            if !votes.is_empty() {
                yield objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk {
                    id: id.clone(),
                    completions: Vec::new(),
                    votes,
                    scores: Vec::new(),
                    weights: Vec::new(),
                    created,
                    swarm: swarm.clone(),
                    object: objectiveai_sdk::vector::completions::response::streaming::Object::VectorCompletionChunk,
                    usage: None,
                };
            }
        }.boxed()
    }

    /// Creates an error response chunk for a failed LLM completion.
    fn llm_create_streaming_vector_error(
        id: String,
        completion_index: u64,
        error: agent::completions::Error,
        upstream: objectiveai_sdk::agent::Upstream,
        created: u64,
        swarm: String,
    ) -> objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk{
        objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk {
            id,
            completions: vec![
                objectiveai_sdk::vector::completions::response::streaming::AgentCompletionChunk {
                    index: completion_index,
                    inner: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
                        error: Some(objectiveai_sdk::error::ResponseError::from(&error)),
                        upstream,
                        ..Default::default()
                    },
                },
            ],
            votes: Vec::new(),
            scores: Vec::new(),
            weights: Vec::new(),
            created,
            swarm,
            object: objectiveai_sdk::vector::completions::response::streaming::Object::VectorCompletionChunk,
            usage: None,
        }
    }
}

/// Extracts text content, logprobs, and tool call arguments from the last assistant message.
fn extract_assistant_content(
    response: &objectiveai_sdk::agent::completions::response::unary::AgentCompletion,
) -> (
    Option<String>,
    Option<objectiveai_sdk::agent::completions::response::Logprobs>,
    Option<String>,
) {
    use objectiveai_sdk::agent::completions::response::unary::Message;

    let mut text = None;
    let mut logprobs = None;
    let mut tool_call_text = None;

    // Find the last assistant message
    for msg in response.messages.iter().rev() {
        if let Message::Assistant(assistant) = msg {
            // Extract text content
            if let Some(content) = &assistant.content {
                text = Some(rich_content_to_string(content));
            }

            // Extract logprobs
            logprobs = assistant.logprobs.clone();

            // Extract tool call arguments (for the "response_key" tool)
            if let Some(tool_calls) = &assistant.tool_calls {
                for tc in tool_calls {
                    match tc {
                        objectiveai_sdk::agent::completions::message::AssistantToolCall::Function { function, .. } => {
                            if function.name == "response_key" {
                                tool_call_text = Some(function.arguments.clone());
                            }
                        }
                    }
                }
            }

            break;
        }
    }

    (text, logprobs, tool_call_text)
}

/// Converts RichContent to a plain string.
fn rich_content_to_string(
    content: &objectiveai_sdk::agent::completions::message::RichContent,
) -> String {
    match content {
        objectiveai_sdk::agent::completions::message::RichContent::Text(
            text,
        ) => text.clone(),
        objectiveai_sdk::agent::completions::message::RichContent::Parts(
            parts,
        ) => {
            let mut result = String::new();
            for part in parts {
                if let objectiveai_sdk::agent::completions::message::RichContentPart::Text { text } = part {
                    result.push_str(text);
                }
            }
            result
        }
    }
}

/// Computes a per-agent seed by hashing the base seed with the agent ID,
/// flat swarm index, prompt ID, and response IDs.
///
/// This ensures each agent in an swarm gets a different but deterministic
/// seed, and different vector completion tasks (with different prompts or
/// responses) also get different seeds for the same agent.
fn per_agent_seed(
    seed: i64,
    agent_instance_hierarchy: &str,
    flat_swarm_index: usize,
    prompt_id: &str,
    responses_ids: &[String],
) -> i64 {
    let mut hasher = twox_hash::XxHash3_64::with_seed(seed as u64);
    hasher.write(agent_instance_hierarchy.as_bytes());
    hasher.write(&(flat_swarm_index as u64).to_le_bytes());
    hasher.write(prompt_id.as_bytes());
    for rid in responses_ids {
        hasher.write(rid.as_bytes());
    }
    hasher.finish() as i64
}

/// Creates an RNG, seeded if a seed is provided (for deterministic results).
fn make_rng(seed: Option<u64>) -> impl Rng {
    match seed {
        Some(seed) => rand::rngs::StdRng::seed_from_u64(seed),
        None => rand::rngs::StdRng::from_os_rng(),
    }
}

/// Transforms messages for vector voting by appending response options to the
/// last user message using `into_parts_for_prompt`.
///
/// For instruction mode, also appends a key listing to the user message.
fn transform_messages_for_vector(
    mut messages: Vec<objectiveai_sdk::agent::completions::message::Message>,
    responses: &[objectiveai_sdk::agent::completions::message::RichContent],
    pfx_indices: &[(String, usize)],
    output_mode: objectiveai_sdk::agent::OutputMode,
) -> Vec<objectiveai_sdk::agent::completions::message::Message> {
    use objectiveai_sdk::agent::completions::message::{
        Message, RichContent, RichContentPart, UserMessage,
    };

    // Build response parts using into_parts_for_prompt
    let response_parts =
        super::vector_responses::into_parts_for_prompt(responses, pfx_indices);

    // Append to the last user message, or create one
    let mut found_user = false;
    for msg in messages.iter_mut().rev() {
        if let Message::User(user_msg) = msg {
            // Convert Text to Parts if needed, then extend
            let parts = match &mut user_msg.content {
                RichContent::Text(text) => {
                    let mut new_parts =
                        Vec::with_capacity(2 + response_parts.len());
                    new_parts
                        .push(RichContentPart::Text { text: text.clone() });
                    user_msg.content = RichContent::Parts(new_parts);
                    match &mut user_msg.content {
                        RichContent::Parts(p) => p,
                        _ => unreachable!(),
                    }
                }
                RichContent::Parts(parts) => parts,
            };
            parts.push(RichContentPart::Text {
                text: if parts.is_empty() {
                    "Select the response:\n\n".to_string()
                } else {
                    "\n\nSelect the response:\n\n".to_string()
                },
            });
            parts.extend(response_parts.clone());

            // For instruction mode, append key listing to the same user message
            if output_mode == objectiveai_sdk::agent::OutputMode::Instruction {
                parts.push(RichContentPart::Text {
                    text: format!(
                        "\n\nOutput one response key including backticks:\n- {}",
                        pfx_indices
                            .iter()
                            .map(|(key, _)| key.clone())
                            .collect::<Vec<_>>()
                            .join("\n- "),
                    ),
                });
            }

            found_user = true;
            break;
        }
    }

    if !found_user {
        let mut parts = Vec::with_capacity(1 + response_parts.len());
        parts.push(RichContentPart::Text {
            text: "Select the response:\n\n".to_string(),
        });
        parts.extend(response_parts);
        if output_mode == objectiveai_sdk::agent::OutputMode::Instruction {
            parts.push(RichContentPart::Text {
                text: format!(
                    "\n\nOutput one response key including backticks:\n- {}",
                    pfx_indices
                        .iter()
                        .map(|(key, _)| key.clone())
                        .collect::<Vec<_>>()
                        .join("\n- "),
                ),
            });
        }
        messages.push(Message::User(UserMessage {
            content: RichContent::Parts(parts),
        }));
    }

    messages
}
