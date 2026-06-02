//! `functions executions create standard` — async handler stub.

use crate::agent::completions::message::Message;  // unused placeholder to keep imports tidy
use crate::cli::command::IntoCommand;
use crate::functions::FullInlineFunctionOrRemoteCommitOptional;
use crate::functions::InlineProfileOrRemoteCommitOptional;
use crate::functions::expression::InputValue;

#[allow(dead_code)]
type _UnusedMessage = Message;

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub function: FullInlineFunctionOrRemoteCommitOptional,
    pub profile: InlineProfileOrRemoteCommitOptional,
    pub input: RequestInput,
    pub continuation: Option<String>,
    pub retry_token: Option<String>,
    pub seed: Option<i64>,
    pub split: bool,
    pub invert: bool,
    pub detach: bool,
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum RequestInput {
    Inline(InputValue),
    PythonInline(String),
    PythonFile(std::path::PathBuf),
}

impl RequestInput {
    fn push_flags(&self, out: &mut Vec<String>) {
        match self {
            RequestInput::Inline(v) => {
                out.push("--input-inline".to_string());
                out.push(serde_json::to_string(v).expect("input serializes"));
            }
            RequestInput::PythonInline(code) => {
                out.push("--input-python-inline".to_string());
                out.push(code.clone());
            }
            RequestInput::PythonFile(p) => {
                out.push("--input-python-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "functions".to_string(),
            "executions".to_string(),
            "create".to_string(),
            "standard".to_string(),
            "--function-inline".to_string(),
            serde_json::to_string(&self.function).expect("function serializes"),
            "--profile-inline".to_string(),
            serde_json::to_string(&self.profile).expect("profile serializes"),
        ];
        self.input.push_flags(&mut argv);
        if let Some(c) = &self.continuation {
            argv.push("--continuation".to_string());
            argv.push(c.clone());
        }
        if let Some(t) = &self.retry_token {
            argv.push("--retry-token".to_string());
            argv.push(t.clone());
        }
        if let Some(seed) = self.seed {
            argv.push("--seed".to_string());
            argv.push(seed.to_string());
        }
        if self.split {
            argv.push("--split".to_string());
        }
        if self.invert {
            argv.push("--invert".to_string());
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

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RequestDangerousAdvanced {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum ResponseItem {
    Chunk(crate::functions::executions::response::streaming::FunctionExecutionChunk),
    Id(String),
}
