use futures::StreamExt;

use super::create_args::CreateArgs;

/// Result item for a single builder agent.
#[derive(serde::Serialize)]
struct ResultItem {
    agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    score: Option<f64>,
    error: Option<objectiveai::error::ResponseError>,
}

pub async fn handle(args: CreateArgs, cli_config: &crate::Config) -> Result<crate::Output, crate::error::Error> {
    args.instructions.verify(cli_config, crate::instructions::InstructionsScope::LaboratoryExecutions)?;

    let mut builder_agents = Vec::with_capacity(args.builder_agent.len());
    for a in &args.builder_agent {
        builder_agents.push(a.clone().resolve(|| async {
            let (_, mut c) = crate::config::read(cli_config).await.unwrap();
            c.agents().get_favorites().to_vec()
        }).await?);
    }

    // Keep original agent refs for the final output (in arg order)
    let original_agents = builder_agents.clone();

    let evaluation_agent = match args.evaluation_agent {
        Some(a) => Some(a.resolve(|| async {
            let (_, mut c) = crate::config::read(cli_config).await.unwrap();
            c.agents().get_favorites().to_vec()
        }).await?),
        None => None,
    };
    let builder_messages = args.builder_messages.resolve()?;
    let evaluation_messages = args.evaluation_messages.resolve()?;
    let evaluation_output_schema = args.evaluation_output_schema.resolve()?;
    let builder_continuation = args.builder_continuation.resolve()?;
    let evaluation_continuation = args.evaluation_continuation.resolve()?;

    let python_code = if let Some(inline) = args.output_python.output_python_inline {
        Some(inline)
    } else if let Some(path) = args.output_python.output_python_file {
        Some(
            std::fs::read_to_string(&path)
                .map_err(|e| crate::error::Error::PythonFileRead(path, e))?,
        )
    } else {
        None
    };

    let num_agents = original_agents.len();

    let params = objectiveai::laboratories::executions::request::LaboratoryExecutionCreateParams {
        docker_image: args.docker_image,
        builder_agents,
        evaluation_agent,
        builder_messages,
        evaluation_messages,
        evaluation_output_schema,
        builder_continuation,
        evaluation_continuation,
        max_evaluation_retries: args.max_evaluation_retries,
        persist: None,
        provider: None,
        seed: args.seed,
        stream: Some(true),
    };

    let fs_client = objectiveai::filesystem::Client::new(cli_config.config_base_dir.as_deref(), None::<String>, None::<String>);
    let log_writer = objectiveai::filesystem::logs::client::write_laboratory_execution(&fs_client);

    crate::api::run(
        Box::new(move |http_client| Box::pin(async move {
            let stream =
                objectiveai::laboratories::executions::create_laboratory_execution_streaming(
                    &http_client, params,
                )
                .await?;

            let accumulated = crate::log_stream::consume_with_coalesced_writes(
                stream.map(|r| r.map_err(crate::error::Error::from)),
                log_writer,
                |agg: &mut objectiveai::laboratories::executions::response::streaming::LaboratoryExecutionChunk, c| agg.push(c),
            ).await?;

            let execution: objectiveai::laboratories::executions::response::unary::LaboratoryExecution =
                accumulated.into();

            // Collect evaluation outputs indexed by agent_index
            // agent_index -> (output, error)
            let mut eval_map: std::collections::HashMap<u64, (Option<&objectiveai::functions::expression::InputValue>, Option<&objectiveai::error::ResponseError>)> =
                std::collections::HashMap::new();
            for eval in &execution.evaluations {
                eval_map.insert(eval.agent_index, (eval.output.as_ref(), eval.inner.error.as_ref()));
            }

            // Collect non-None outputs in agent_index order, tracking which indices have outputs
            let mut outputs_with_indices: Vec<(u64, &objectiveai::functions::expression::InputValue)> = Vec::new();
            for agent_index in 0..num_agents as u64 {
                if let Some((Some(output), _)) = eval_map.get(&agent_index) {
                    outputs_with_indices.push((agent_index, output));
                }
            }

            // Run python scoring if there are any outputs and a script was provided
            let mut score_map: std::collections::HashMap<u64, f64> = std::collections::HashMap::new();
            if !outputs_with_indices.is_empty() {
                if let Some(ref script) = python_code {
                    // Pass evaluations as sys.argv[1], script reads via:
                    //   import json, sys; evaluations = json.loads(sys.argv[1])
                    let evaluations_json = serde_json::to_string(
                        &outputs_with_indices
                            .iter()
                            .map(|(_, output)| serde_json::to_value(output).unwrap())
                            .collect::<Vec<_>>(),
                    )
                    .unwrap();

                    let scores: Vec<f64> = crate::python::exec_code_with_args(
                        script,
                        &[evaluations_json],
                    )?;

                    if scores.len() < outputs_with_indices.len() {
                        return Err(crate::error::Error::MissingArgs(
                            "python script returned fewer scores than evaluation outputs",
                        ));
                    }

                    for (i, (agent_index, _)) in outputs_with_indices.iter().enumerate() {
                        score_map.insert(*agent_index, scores[i]);
                    }
                }
            }

            // Build results in original argument order
            let results: Vec<ResultItem> = (0..num_agents)
                .map(|i| {
                    let agent_index = i as u64;
                    let agent = original_agents[i].clone();
                    let score = score_map.get(&agent_index).copied();
                    let error = eval_map
                        .get(&agent_index)
                        .and_then(|(_, e)| *e)
                        .cloned();
                    ResultItem {
                        agent,
                        score,
                        error,
                    }
                })
                .collect();

            Ok(serde_json::to_string(&results).unwrap())
        })),
        true,
    )
    .await
}
