//! `functions inventions recursive create remote` — async handler stub.

use crate::cli::command::CommandRequest;
use crate::cli::command::agents::spawn::AgentSpec;
use crate::functions::inventions::state::ParamsState;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub state: RequestState,
    pub agent: AgentSpec,
    pub continuation: Option<String>,
    pub seed: Option<i64>,
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum RequestState {
    Inline(ParamsState),
    Ref(String),
}

impl RequestState {
    fn push_flags(&self, out: &mut Vec<String>) {
        match self {
            RequestState::Inline(v) => {
                out.push("--state-inline".to_string());
                out.push(serde_json::to_string(v).expect("state serializes"));
            }
            RequestState::Ref(r) => {
                out.push("--state".to_string());
                out.push(r.clone());
            }
        }
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "functions".to_string(),
            "inventions".to_string(),
            "recursive".to_string(),
            "create".to_string(),
            "remote".to_string(),
        ];
        self.state.push_flags(&mut argv);
        argv.push("--agent-inline".to_string());
        argv.push(serde_json::to_string(&self.agent).expect("agent serializes"));
        if let Some(c) = &self.continuation {
            argv.push("--continuation".to_string());
            argv.push(c.clone());
        }
        if let Some(seed) = self.seed {
            argv.push("--seed".to_string());
            argv.push(seed.to_string());
        }
        if let Some(advanced) = &self.dangerous_advanced {
            argv.push("--dangerous-advanced".to_string());
            argv.push(
                serde_json::to_string(advanced)
                    .expect("RequestDangerousAdvanced serializes"),
            );
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RequestDangerousAdvanced {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum ResponseItem {
    Chunk(crate::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk),
    Id(String),
}

/// Non-chunk variant of [`ResponseItem`]. Returned by the unary `execute`
/// path (with `dangerous_advanced.stream` cleared) when the cli emits a
/// single bare id string.
pub type Response = String;

#[derive(clap::Args)]
#[group(id = "state", required = true, multiple = false)]
pub struct Args {
    /// State reference.
    #[arg(long, group = "state")]
    pub state: Option<String>,
    /// Inline JSON state.
    #[arg(long, group = "state")]
    pub state_inline: Option<String>,
    /// Inline JSON agent definition.
    #[arg(long)]
    pub agent_inline: String,
    /// Continuation token from a previous response.
    #[arg(long)]
    pub continuation: Option<String>,
    /// Seed for deterministic mock responses.
    #[arg(long)]
    pub seed: Option<i64>,
    /// Advanced opt-in flags as inline JSON.
    #[arg(long)]
    pub dangerous_advanced: Option<String>,
    /// jq filter applied to the JSON output.
    #[arg(long)]
    pub jq: Option<String>,
}

#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Command {
    #[command(flatten)]
    pub args: Args,
    #[command(subcommand)]
    pub schema: Option<Schema>,
}

#[derive(clap::Subcommand)]
pub enum Schema {
    /// Emit the JSON Schema for this leaf's `Request` type and exit.
    RequestSchema(request_schema::Args),
    /// Emit the JSON Schema for this leaf's `Response` type and exit.
    ResponseSchema(response_schema::Args),
}

impl TryFrom<Args> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(args: Args) -> Result<Self, Self::Error> {
        let state = if let Some(s) = args.state_inline {
            let mut de = serde_json::Deserializer::from_str(&s);
            let v = serde_path_to_error::deserialize(&mut de).map_err(|source| {
                crate::cli::command::FromArgsError {
                    field: "state_inline",
                    source: source.into(),
                }
            })?;
            RequestState::Inline(v)
        } else {
            RequestState::Ref(args.state.unwrap())
        };
        let agent = {
            let mut de = serde_json::Deserializer::from_str(&args.agent_inline);
            serde_path_to_error::deserialize(&mut de).map_err(|source| {
                crate::cli::command::FromArgsError {
                    field: "agent_inline",
                    source: source.into(),
                }
            })?
        };
        let dangerous_advanced = if let Some(s) = args.dangerous_advanced {
            let mut de = serde_json::Deserializer::from_str(&s);
            let v = serde_path_to_error::deserialize(&mut de).map_err(|source| {
                crate::cli::command::FromArgsError {
                    field: "dangerous_advanced",
                    source: source.into(),
                }
            })?;
            Some(v)
        } else {
            None
        };
        Ok(Self {
            state,
            agent,
            continuation: args.continuation,
            seed: args.seed,
            dangerous_advanced,
            jq: args.jq,
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute_streaming<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.jq = None;
    let mut advanced = request.dangerous_advanced.unwrap_or_default();
    advanced.stream = Some(true);
    request.dangerous_advanced = Some(advanced);
    executor.execute(request).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_streaming_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,
) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.jq = Some(jq);
    let mut advanced = request.dangerous_advanced.unwrap_or_default();
    advanced.stream = Some(true);
    request.dangerous_advanced = Some(advanced);
    executor.execute(request).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
) -> Result<Response, E::Error> {
    request.jq = None;
    if let Some(advanced) = request.dangerous_advanced.as_mut() {
        advanced.stream = None;
    }
    executor.execute_one(request).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,
) -> Result<serde_json::Value, E::Error> {
    request.jq = Some(jq);
    if let Some(advanced) = request.dangerous_advanced.as_mut() {
        advanced.stream = None;
    }
    executor.execute_one(request).await
}

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub jq: Option<String>,
    }

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["functions", "inventions", "recursive", "create", "remote", "request-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;

    impl TryFrom<Args> for Request {
        type Error = crate::cli::command::FromArgsError;
        fn try_from(args: Args) -> Result<Self, Self::Error> {
            Ok(Self { jq: args.jq })
        }
    }

    #[cfg(feature = "cli-executor")]
    pub async fn execute<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
    ) -> Result<Response, E::Error> {
        request.jq = None;
        executor.execute_one(request).await
    }

    #[cfg(feature = "cli-executor")]
    pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
        jq: String,
    ) -> Result<serde_json::Value, E::Error> {
        request.jq = Some(jq);
        executor.execute_one(request).await
    }
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub jq: Option<String>,
    }

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["functions", "inventions", "recursive", "create", "remote", "response-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;

    impl TryFrom<Args> for Request {
        type Error = crate::cli::command::FromArgsError;
        fn try_from(args: Args) -> Result<Self, Self::Error> {
            Ok(Self { jq: args.jq })
        }
    }

    #[cfg(feature = "cli-executor")]
    pub async fn execute<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
    ) -> Result<Response, E::Error> {
        request.jq = None;
        executor.execute_one(request).await
    }

    #[cfg(feature = "cli-executor")]
    pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
        jq: String,
    ) -> Result<serde_json::Value, E::Error> {
        request.jq = Some(jq);
        executor.execute_one(request).await
    }
}
