//! `agents message` — async handler stub.

use crate::agent::completions::message::RichContent;
use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub agent_instance_hierarchy: String,
    pub message: RequestMessage,
    pub seed: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum RequestMessage {
    Inline(RichContent),
    Simple(String),
    File(std::path::PathBuf),
    PythonInline(String),
    PythonFile(std::path::PathBuf),
}

impl RequestMessage {
    fn push_flags(&self, out: &mut Vec<String>) {
        match self {
            RequestMessage::Inline(rich) => {
                out.push("--inline".to_string());
                out.push(
                    serde_json::to_string(rich)
                        .expect("RichContent serializes to JSON cleanly"),
                );
            }
            RequestMessage::Simple(s) => {
                out.push("--simple".to_string());
                out.push(s.clone());
            }
            RequestMessage::File(p) => {
                out.push("--file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
            RequestMessage::PythonInline(code) => {
                out.push("--python-inline".to_string());
                out.push(code.clone());
            }
            RequestMessage::PythonFile(p) => {
                out.push("--python-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "message".to_string(),
            self.agent_instance_hierarchy.clone(),
        ];
        self.message.push_flags(&mut argv);
        if let Some(seed) = self.seed {
            argv.push("--seed".to_string());
            argv.push(seed.to_string());
        }
        argv
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum Response {
    Delivered { agent_id: String },
    Queued { agent_id: String, response_id: String },
}

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
