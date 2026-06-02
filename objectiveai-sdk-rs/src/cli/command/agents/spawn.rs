//! `agents spawn` — async handler stub.

use crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use crate::agent::completions::message::Message;
use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub prompt: RequestPrompt,
    pub agent: InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    pub seed: Option<i64>,
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum RequestPrompt {
    Inline(Vec<Message>),
    Simple(String),
    File(std::path::PathBuf),
    PythonInline(String),
    PythonFile(std::path::PathBuf),
}

impl RequestPrompt {
    fn push_flags(&self, out: &mut Vec<String>) {
        match self {
            RequestPrompt::Inline(msgs) => {
                out.push("--inline".to_string());
                out.push(
                    serde_json::to_string(msgs).expect("Vec<Message> serializes"),
                );
            }
            RequestPrompt::Simple(s) => {
                out.push("--simple".to_string());
                out.push(s.clone());
            }
            RequestPrompt::File(p) => {
                out.push("--file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
            RequestPrompt::PythonInline(code) => {
                out.push("--python-inline".to_string());
                out.push(code.clone());
            }
            RequestPrompt::PythonFile(p) => {
                out.push("--python-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["agents".to_string(), "spawn".to_string()];
        self.prompt.push_flags(&mut argv);
        // The cli's `AgentArg` (from `define_inline_or_ref!`) accepts
        // either `--agent <REFERENCE>` (a `FavoriteRef` wire form, then
        // resolved to the `Remote` variant) or `--agent-inline <JSON>`
        // (deserialized directly into the SDK type). We always emit the
        // inline form because the Request already holds the resolved
        // typed value — the cli's resolve hits the inline branch and
        // round-trips identically for both Inline and Remote variants.
        argv.push("--agent-inline".to_string());
        argv.push(
            serde_json::to_string(&self.agent)
                .expect("InlineAgentBaseWithFallbacksOrRemoteCommitOptional serializes"),
        );
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
    Chunk(crate::agent::completions::response::streaming::AgentCompletionChunk),
    Id(String),
}

#[derive(clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub prompt: PromptArgs,
    #[command(flatten)]
    pub agent: AgentArgs,
    /// Seed for deterministic mock responses.
    #[arg(long)]
    pub seed: Option<i64>,
    /// Raw JSON for `RequestDangerousAdvanced` (e.g. `{"stream":true}`).
    #[arg(long)]
    pub dangerous_advanced: Option<String>,
    /// jq filter applied to the JSON output.
    #[arg(long)]
    pub jq: Option<String>,
}

#[derive(clap::Args)]
#[group(required = true, multiple = false)]
pub struct PromptArgs {
    /// Plain text — becomes one user message.
    #[arg(long)]
    pub simple: Option<String>,
    /// Inline JSON messages array.
    #[arg(long)]
    pub inline: Option<String>,
    /// Path to a JSON file containing the messages array.
    #[arg(long)]
    pub file: Option<std::path::PathBuf>,
    /// Inline Python code that produces the messages array.
    #[arg(long)]
    pub python_inline: Option<String>,
    /// Path to a Python file that produces the messages array.
    #[arg(long)]
    pub python_file: Option<std::path::PathBuf>,
}

#[derive(clap::Args)]
#[group(required = true, multiple = false)]
pub struct AgentArgs {
    /// Favorite-ref or remote-path string.
    #[arg(long)]
    pub agent: Option<String>,
    /// Inline JSON for the full agent definition.
    #[arg(long)]
    pub agent_inline: Option<String>,
}

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub jq: Option<String>,
    }

    #[derive(clap::Args)]
    pub struct Args {
        #[arg(long)]
        pub jq: Option<String>,
    }

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["agents", "spawn", "--request-schema"].into_iter().map(String::from).collect();
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

    #[derive(clap::Args)]
    pub struct Args {
        #[arg(long)]
        pub jq: Option<String>,
    }

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["agents", "spawn", "--response-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;
}
