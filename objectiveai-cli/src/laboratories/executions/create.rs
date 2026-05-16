use futures::StreamExt;
use objectiveai_sdk::cli::output::{LabResultItem, Laboratory};

use super::create_args::CreateArgs;

pub async fn handle(
    args: CreateArgs,
    cli_config: &crate::Config,
    handle: &objectiveai_sdk::cli::output::Handle,
) -> Result<(), crate::error::Error> {
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

    let params = objectiveai_sdk::laboratories::executions::request::LaboratoryExecutionCreateParams {
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

    let fs_client = objectiveai_sdk::filesystem::Client::new(cli_config.config_base_dir.as_deref(), None::<String>, None::<String>);
    let log_writer = fs_client.write_laboratory_execution();

    let handle = handle.clone();
    crate::api::run(
        Box::new(move |http_client| Box::pin(async move {
            let stream =
                objectiveai_sdk::laboratories::executions::create_laboratory_execution_streaming(
                    &http_client, params,
                )
                .await?;

            // Emit each chunk's inner errors live (Warn) before pushing.
            let emit_handle = handle.clone();
            let stream = stream.then(move |result| {
                let handle = emit_handle.clone();
                async move {
                    if let Ok(chunk) = &result {
                        for inner in chunk.inner_errors() {
                            objectiveai_sdk::cli::output::Output::<serde_json::Value>::Error(
                                objectiveai_sdk::cli::output::Error {
                                    level: objectiveai_sdk::cli::output::Level::Warn,
                                    fatal: false,
                                    message: serde_json::to_value(&inner).unwrap(),
                                },
                            )
                            .emit(&handle)
                            .await;
                        }
                    }
                    result.map_err(crate::error::Error::from)
                }
            });

            let mut accumulated = crate::log_stream::consume_with_coalesced_writes(
                stream,
                log_writer,
                |agg: &mut objectiveai_sdk::laboratories::executions::response::streaming::LaboratoryExecutionChunk, c| agg.push(c),
                handle.clone(),
            ).await?;

            if let Some(error) = accumulated.error.take() {
                return Err(crate::error::Error::ResponseError(error));
            }

            let execution: objectiveai_sdk::laboratories::executions::response::unary::LaboratoryExecution =
                accumulated.into();

            // Collect evaluation outputs indexed by agent_index. Per-evaluation
            // errors were already streamed as Output::Error during the stream,
            // so we only need outputs here.
            let mut eval_map: std::collections::HashMap<u64, Option<&objectiveai_sdk::functions::expression::InputValue>> =
                std::collections::HashMap::new();
            for eval in &execution.evaluations {
                eval_map.insert(eval.agent_index, eval.output.as_ref());
            }

            // Collect non-None outputs in agent_index order, tracking which indices have outputs
            let mut outputs_with_indices: Vec<(u64, &objectiveai_sdk::functions::expression::InputValue)> = Vec::new();
            for agent_index in 0..num_agents as u64 {
                if let Some(Some(output)) = eval_map.get(&agent_index) {
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

            // Build results in original argument order. Per-evaluation
            // failures already surfaced as Output::Error during streaming;
            // `score: None` covers both "failed" and "no scoreable output."
            let results: Vec<LabResultItem> = (0..num_agents)
                .map(|i| {
                    let agent_index = i as u64;
                    let agent = original_agents[i].clone();
                    let score = score_map.get(&agent_index).copied();
                    LabResultItem { agent, score }
                })
                .collect();

            objectiveai_sdk::cli::output::Output::<Laboratory>::Notification(objectiveai_sdk::cli::output::Notification { value: 
                Laboratory { laboratory: results },
             })
            .emit(&handle).await;
            Ok(())
        })),
        true,
    )
    .await
}
