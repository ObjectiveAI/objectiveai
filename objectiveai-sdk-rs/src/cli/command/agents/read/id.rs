//! `agents read id` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub id: i64,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        vec![
            "agents".to_string(),
            "read".to_string(),
            "id".to_string(),
            self.id.to_string(),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Response {
    // Agent-completion typed envelopes / metadata
    AgentCompletion {
        content: crate::agent::completions::response::streaming::AgentCompletionChunkLog,
    },
    AgentCompletionRequest {
        content: crate::agent::completions::request::AgentCompletionCreateParamsLog,
    },
    Message { content: crate::agent::completions::message::MessageLog },
    Logprobs { content: crate::agent::completions::response::Logprobs },
    ToolCall { content: crate::agent::completions::message::AssistantToolCallDelta },

    // Vector / function typed envelopes
    VectorCompletion {
        content: crate::vector::completions::response::streaming::VectorCompletionChunkLog,
    },
    VectorCompletionRequest {
        content: crate::vector::completions::request::VectorCompletionCreateParams,
    },
    FunctionExecution {
        content: crate::functions::executions::response::streaming::FunctionExecutionChunkLog,
    },
    FunctionExecutionRequest {
        content: crate::functions::executions::request::FunctionExecutionCreateParamsLog,
    },
    FunctionInvention {
        content: crate::functions::inventions::response::streaming::FunctionInventionChunkLog,
    },
    FunctionInventionRequest {
        content: crate::functions::inventions::request::FunctionInventionCreateParams,
    },
    FunctionInventionRecursive {
        content: crate::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunkLog,
    },
    FunctionInventionRecursiveRequest {
        content: crate::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParamsLog,
    },

    // Collapsed text/media — one variant per content kind, regardless
    // of where the file lives (continuation, reasoning, refusal, every
    // message/tool/request/notification text and retry_token all surface
    // as Text; every image as Image; etc.).
    Text { text: String },
    Image { image_url: crate::agent::completions::message::ImageUrl },
    Audio { input_audio: crate::agent::completions::message::InputAudio },
    Video { video_url: crate::agent::completions::message::VideoUrl },
    File { file: crate::agent::completions::message::File },
}

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["agents", "read", "id", "--request-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["agents", "read", "id", "--response-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}
