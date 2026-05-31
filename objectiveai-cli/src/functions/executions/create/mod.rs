use clap::{Args, Subcommand};

crate::define_inline_or_ref!(FunctionArg, "function", objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional, Remote);
crate::define_inline_or_ref!(ProfileArg, "profile", objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional, Remote);

/// How input is provided to the function execution.
#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct InputSource {
    /// Inline JSON input value
    #[arg(long)]
    input_inline: Option<String>,
    /// Inline Python code that produces the input value
    #[arg(long)]
    input_python_inline: Option<String>,
    /// Path to a Python file that produces the input value
    #[arg(long)]
    input_python_file: Option<std::path::PathBuf>,
}

impl InputSource {
    fn resolve(self) -> Result<objectiveai_sdk::functions::expression::InputValue, crate::error::Error> {
        if let Some(inline) = self.input_inline {
            let mut de = serde_json::Deserializer::from_str(&inline);
            return serde_path_to_error::deserialize(&mut de)
                .map_err(crate::error::Error::InlineDeserialize);
        }
        if let Some(code) = self.input_python_inline {
            return crate::python::exec_code(&code);
        }
        if let Some(path) = self.input_python_file {
            return crate::python::exec_file(&path);
        }
        unreachable!("clap group ensures one is set")
    }
}

use objectiveai_sdk::cli::output::{Execution, ExecutionResult};

#[derive(Subcommand)]
pub enum Commands {
    /// Standard execution strategy (scalar or vector)
    Standard {
        #[command(flatten)]
        function: FunctionArg,
        #[command(flatten)]
        profile: ProfileArg,
        #[command(flatten)]
        input: InputSource,
        #[command(flatten)]
        continuation: crate::continuation::ContinuationArgs,
        /// Retry token from a previous execution
        #[arg(long)]
        retry_token: Option<String>,
        /// Seed for deterministic mock responses
        #[arg(long)]
        seed: Option<i64>,
        /// Treat input as an array and execute once per element
        #[arg(long)]
        split: bool,
        /// Invert every output in the response (scalar: 1-x; vector:
        /// rank-invert so the highest-scoring position becomes the
        /// lowest, etc.). Applied after expressions evaluate.
        #[arg(long)]
        invert: bool,
        /// Run in the background: print PID and log path, then exit
        #[arg(long)]
        detach: bool,
    },
    /// Swiss System tournament strategy (vector only)
    SwissSystem {
        #[command(flatten)]
        function: FunctionArg,
        #[command(flatten)]
        profile: ProfileArg,
        #[command(flatten)]
        input: InputSource,
        #[command(flatten)]
        continuation: crate::continuation::ContinuationArgs,
        /// Retry token from a previous execution
        #[arg(long)]
        retry_token: Option<String>,
        /// Seed for deterministic mock responses
        #[arg(long)]
        seed: Option<i64>,
        /// Treat input as an array and execute once per element
        #[arg(long)]
        split: bool,
        /// Invert every output in the response (scalar: 1-x; vector:
        /// rank-invert so the highest-scoring position becomes the
        /// lowest, etc.). Applied after expressions evaluate.
        #[arg(long)]
        invert: bool,
        /// How many vector responses per execution (default 10)
        #[arg(long)]
        pool: Option<usize>,
        /// How many sequential rounds of comparison (default 3)
        #[arg(long)]
        rounds: Option<usize>,
        /// Run in the background: print PID and log path, then exit
        #[arg(long)]
        detach: bool,
    },
}

async fn fn_favorites(cli_config: &crate::Config) -> Vec<objectiveai_sdk::filesystem::config::Favorite> {
    let (_, mut config) = crate::config::read(cli_config).await.unwrap();
    config.functions().get_favorites().to_vec()
}

async fn profile_favorites(cli_config: &crate::Config) -> Vec<objectiveai_sdk::filesystem::config::Favorite> {
    let (_, mut config) = crate::config::read(cli_config).await.unwrap();
    config.functions().profiles().get_favorites().to_vec()
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        let (function_source, profile_source, input_source, continuation_args, retry_token, seed, split, invert, strategy, detach) = match self {
            Commands::Standard { function, profile, input, continuation, retry_token, seed, split, invert, detach } => {
                (function, profile, input, continuation, retry_token, seed, split, invert, objectiveai_sdk::functions::executions::request::Strategy::Default, detach)
            }
            Commands::SwissSystem { function, profile, input, continuation, retry_token, seed, split, invert, pool, rounds, detach } => {
                let strategy = objectiveai_sdk::functions::executions::request::Strategy::SwissSystem { pool, rounds };
                (function, profile, input, continuation, retry_token, seed, split, invert, strategy, detach)
            }
        };

        if detach {
            crate::api::detach::detach(handle).await;
        }

        let function = function_source.resolve(|| fn_favorites(cli_config)).await?;
        let profile = profile_source.resolve(|| profile_favorites(cli_config)).await?;
        let input_value = input_source.resolve()?;
        let continuation = continuation_args.resolve()?;

        let params = objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams {
            function,
            profile,
            retry_token,
            from_cache: None,
            reasoning: None,
            strategy: Some(strategy),
            input: input_value,
            split: if split { Some(true) } else { None },
            invert: if invert { Some(true) } else { None },
            provider: None,
            seed,
            stream: Some(true),
            continuation,
        };

        // Delegate to cli-stream. Inner errors per chunk are surfaced
        // by `stream_subprocess::run` as `Output::Error(Warn)` via the
        // closure that lists them.
        let aggregate = crate::api::stream_subprocess::run::<
            objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk,
        >(
            cli_config,
            &["functions", "executions", "create"],
            &params,
            handle,
            |chunk| chunk.inner_errors().map(|e| serde_json::to_value(&e).unwrap()).collect(),
            |agg, c| agg.push(c),
        ).await?;
        let mut chunk = aggregate.ok_or(crate::error::Error::EmptyStream)?;

        // Root-level execution failure -> propagate as Err so the
        // global path emits a single Output::Error with Level::Error
        // and fatal=true, exit code 1.
        if let Some(error) = chunk.error.take() {
            return Err(crate::error::Error::ResponseError(error));
        }

        // Extract output (default to Err { error: null } if missing)
        let output = chunk.output
            .map(|o| o.unwrap())
            .unwrap_or(objectiveai_sdk::functions::expression::TaskOutputOwned::Err {
                error: serde_json::Value::Null,
            });

        let result = ExecutionResult { output };
        objectiveai_sdk::cli::output::Output::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: (Execution { execution: result }).into(),
         })
        .emit(handle).await;
        Ok(())
    }
}
