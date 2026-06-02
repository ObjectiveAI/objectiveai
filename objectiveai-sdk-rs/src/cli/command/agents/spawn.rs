//! `agents spawn` — async handler stub.

use crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use crate::agent::completions::message::Message;
use crate::cli::command::IntoCommand;

pub struct Request {
    pub prompt: RequestPrompt,
    pub agent: InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    pub seed: Option<i64>,
}

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

impl IntoCommand for Request {
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
        argv
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub agent_instance: String,
}
