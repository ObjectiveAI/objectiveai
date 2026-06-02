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
        // TODO: serialize the agent field to whatever flag-set the cli's
        // `AgentArg` macro accepts (--inline / --remote / variants).
        // Deferred — fix during the next compile-pass batch.
        todo!("agents spawn: emit agent flags");
        #[allow(unreachable_code)]
        {
            if let Some(seed) = self.seed {
                argv.push("--seed".to_string());
                argv.push(seed.to_string());
            }
            argv
        }
    }
}
