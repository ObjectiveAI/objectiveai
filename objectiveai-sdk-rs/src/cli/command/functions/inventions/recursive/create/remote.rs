//! `functions inventions recursive create remote` — async handler stub.

use crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use crate::cli::command::CommandRequest;
use crate::functions::inventions::state::ParamsState;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub state: RequestState,
    pub agent: InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    pub continuation: Option<String>,
    pub seed: Option<i64>,
    pub detach: bool,
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
        if self.detach {
            argv.push("--detach".to_string());
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
    /// Run in the background; print PID and log path then exit.
    #[arg(long)]
    pub detach: bool,
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
}
