//! `functions inventions recursive create remote` — async handler stub.

use crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use crate::cli::command::IntoCommand;
use crate::functions::inventions::state::ParamsState;

pub struct Request {
    pub state: RequestState,
    pub agent: InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    pub continuation: Option<String>,
    pub seed: Option<i64>,
    pub detach: bool,
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
}

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

impl IntoCommand for Request {
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
        argv
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RequestDangerousAdvanced {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ResponseItem {
    Chunk(crate::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk),
    Id(String),
}
