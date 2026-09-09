//! The entry's lines, as the container's chunks.

use diverge_provider_sdk::shared::containers::run_loop::response::{
    AgenticLoopChunk, AssistantTextContentChunk, AssistantToolCallChunk, NotificationChunk,
    ToolResponseChunk, UsageChunk, UserChunk,
};

use super::protocol::Response;

/// A response sorted: a chunk to yield, or a control line the runner
/// acts on (`ready`, `done`, `value`, `stopped`, `fatal`).
pub enum Converted {
    /// One chunk, on the main thread.
    Chunk(AgenticLoopChunk),
    /// A line that is the runner's, not the stream's.
    Control(Response),
}

/// Sort one response. The five stream lines convert mechanically —
/// the text delta, the tool call with the id and the arguments the
/// entry gave, the tool result with MCP's own result flattened in, the
/// usage delta, and a notification, never fatal from here (the run's
/// deaths are the runner's to declare). Everything else is control.
pub fn convert(response: Response) -> Converted {
    match response {
        Response::Text { delta } => Converted::Chunk(text(delta)),
        Response::ToolCall {
            id,
            name,
            arguments,
        } => Converted::Chunk(AgenticLoopChunk::AssistantToolCall(
            AssistantToolCallChunk {
                r#type: Default::default(),
                parent_tool_call_id: None,
                id,
                meta: None,
                name,
                arguments: Some(arguments),
            },
        )),
        Response::ToolResult { id, result } => {
            Converted::Chunk(AgenticLoopChunk::ToolResponse(ToolResponseChunk {
                r#type: Default::default(),
                parent_tool_call_id: None,
                id,
                inner: result,
            }))
        }
        Response::Usage {
            prompt,
            completion,
            total,
        } => Converted::Chunk(AgenticLoopChunk::Usage(UsageChunk {
            r#type: Default::default(),
            completion_tokens: completion,
            prompt_tokens: prompt,
            total_tokens: total,
            meta: None,
        })),
        Response::Notification { message } => Converted::Chunk(notification(message, false)),
        control => Converted::Control(control),
    }
}

/// An assistant text chunk, on the main thread.
pub fn text(text: String) -> AgenticLoopChunk {
    AgenticLoopChunk::AssistantTextContent(AssistantTextContentChunk {
        r#type: Default::default(),
        parent_tool_call_id: None,
        logprobs: None,
        inner: rmcp::model::TextContent::new(text),
    })
}

/// A `user` chunk: a queued prompt, at the position it landed.
pub fn user(prompt: String) -> AgenticLoopChunk {
    AgenticLoopChunk::User(UserChunk {
        r#type: Default::default(),
        prompt,
        meta: None,
    })
}

/// A notification chunk, its fatality the caller's verdict.
pub fn notification(message: serde_json::Value, is_fatal: bool) -> AgenticLoopChunk {
    AgenticLoopChunk::Notification(NotificationChunk {
        r#type: Default::default(),
        is_fatal,
        message,
        meta: None,
    })
}

/// A control line where a chunk was expected, as a notification: the
/// entry and this program disagree, which is a bug to see, not to die
/// of.
pub fn unexpected(response: &Response) -> AgenticLoopChunk {
    notification(
        serde_json::json!({
            "kind": "protocol",
            "error": "the entry wrote a line this program did not expect here",
            "line": format!("{response:?}"),
        }),
        false,
    )
}
