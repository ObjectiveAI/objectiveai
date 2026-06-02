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
#[serde(untagged)]
pub enum Response {
    // Typed log envelopes — each variant name is the PascalCase form
    // of its full leaf path under `logs/`, and the payload aliases the
    // matching `… ::get::Response` so the type stays in one place.
    AgentsCompletionsResponse(crate::cli::command::logs::agents::completions::response::get::Response),
    AgentsCompletionsRequest(crate::cli::command::logs::agents::completions::request::get::Response),
    AgentsCompletionsResponseMessages(crate::cli::command::logs::agents::completions::response::messages::get::Response),
    AgentsCompletionsResponseMessagesLogprobs(crate::cli::command::logs::agents::completions::response::messages::logprobs::get::Response),
    AgentsCompletionsResponseMessagesToolCalls(crate::cli::command::logs::agents::completions::response::messages::tool_calls::get::Response),

    VectorCompletionsResponse(crate::cli::command::logs::vector::completions::response::get::Response),
    VectorCompletionsRequest(crate::cli::command::logs::vector::completions::request::get::Response),

    FunctionsExecutionsResponse(crate::cli::command::logs::functions::executions::response::get::Response),
    FunctionsExecutionsRequest(crate::cli::command::logs::functions::executions::request::get::Response),

    FunctionsInventionsResponse(crate::cli::command::logs::functions::inventions::response::get::Response),
    FunctionsInventionsRequest(crate::cli::command::logs::functions::inventions::request::get::Response),

    FunctionsInventionsRecursiveResponse(crate::cli::command::logs::functions::inventions::recursive::response::get::Response),
    FunctionsInventionsRecursiveRequest(crate::cli::command::logs::functions::inventions::recursive::request::get::Response),

    // Collapsed text/media — one variant per content kind, regardless
    // of where the file lives. Untagged tuple variants, no wrapper keys.
    Text(String),
    Image(crate::agent::completions::message::ImageUrl),
    Audio(crate::agent::completions::message::InputAudio),
    Video(crate::agent::completions::message::VideoUrl),
    File(crate::agent::completions::message::File),
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
