//! Codex's events, as the container's chunks.

use std::collections::BTreeMap;

use diverge_provider_sdk::endpoints::containers::agents::run::server::response::{
    AgenticLoopChunk, AssistantReasoningChunk, AssistantTextContentChunk,
    AssistantToolCallChunk, NotificationChunk, ToolResponseChunk, UsageChunk, UserChunk,
};
use rmcp::model::{CallToolResult, ContentBlock, MetaObject};
use serde_json::Value;

use crate::response::item::{
    CommandExecution, CommandExecutionStatus, FileChange, Item, ItemDetails, McpToolCall,
    McpToolCallStatus, PatchApplyStatus, WebSearch,
};
use crate::response::{ThreadEvent, Usage};

/// One process's conversion state — one turn's.
///
/// Codex's item ids are `item_<n>`, minted per process, so they
/// restart every turn and collide across a run; the stream needs an
/// id unique across the run, so every tool call gets one minted here
/// (uuid v4), mapped from the item id for the process's life. A
/// completed item that never started — agent messages and reasoning
/// always, and anything reconciled at the turn's end — gets a fresh
/// id and, for a tool, its call right before its response.
pub struct Turn {
    /// Open calls: the item id, and the id minted for it.
    calls: BTreeMap<String, String>,
    /// The thread the process named, for the continuation.
    pub thread_id: Option<String>,
    /// Whether `turn.started` came.
    pub started: bool,
    /// Whether `turn.completed` or `turn.failed` came.
    pub terminal: bool,
    /// Whether the turn failed.
    pub failed: bool,
    /// The thread's cumulative usage as last reported: the baseline
    /// the next delta is billed from, and the thread row's after the
    /// turn.
    pub baseline: Usage,
}

impl Turn {
    /// A turn, billing from the baseline the thread row holds.
    pub fn new(baseline: Usage) -> Self {
        Turn {
            calls: BTreeMap::new(),
            thread_id: None,
            started: false,
            terminal: false,
            failed: false,
            baseline,
        }
    }

    /// The chunks one event becomes. Zero, one or several.
    pub fn convert(&mut self, event: ThreadEvent) -> Vec<AgenticLoopChunk> {
        match event {
            ThreadEvent::ThreadStarted(started) => {
                self.thread_id = Some(started.thread_id);
                Vec::new()
            }
            ThreadEvent::TurnStarted(_) => {
                self.started = true;
                Vec::new()
            }
            ThreadEvent::TurnCompleted(completed) => {
                self.terminal = true;
                vec![self.usage(completed.usage)]
            }
            ThreadEvent::TurnFailed(failed) => {
                self.terminal = true;
                self.failed = true;
                vec![notification(
                    serde_json::json!({ "kind": "turn", "error": failed.error.message }),
                    true,
                )]
            }
            ThreadEvent::Error(error) => vec![notification(
                serde_json::json!({ "kind": "codex", "error": error.message }),
                false,
            )],
            ThreadEvent::ItemStarted(started) => self.started_item(started.item),
            ThreadEvent::ItemUpdated(updated) => self.updated_item(updated.item),
            ThreadEvent::ItemCompleted(completed) => self.completed_item(completed.item),
        }
    }

    /// An item beginning: a tool call under a minted id, or news.
    fn started_item(&mut self, item: Item) -> Vec<AgenticLoopChunk> {
        match &item.details {
            ItemDetails::CommandExecution(command) => {
                let id = self.open(&item.id);
                vec![call(id, "command_execution", command_arguments(command))]
            }
            ItemDetails::FileChange(change) => {
                let id = self.open(&item.id);
                vec![call(id, "file_change", file_change_arguments(change))]
            }
            ItemDetails::McpToolCall(mcp) => {
                let id = self.open(&item.id);
                vec![call(id, &mcp_name(mcp), mcp.arguments.to_string())]
            }
            ItemDetails::WebSearch(search) => {
                let id = self.open(&item.id);
                vec![call(id, "web_search", web_search_arguments(search))]
            }
            ItemDetails::TodoList(list) => vec![notification(
                serde_json::json!({ "kind": "todo_list", "items": list.items }),
                false,
            )],
            ItemDetails::CollabToolCall(collab) => vec![notification(
                serde_json::json!({ "kind": "collab", "call": collab }),
                false,
            )],
            // Never started by the source; said whole when completed.
            ItemDetails::AgentMessage(_) | ItemDetails::Reasoning(_) | ItemDetails::Error(_) => {
                Vec::new()
            }
        }
    }

    /// An item changing: only the todo list does.
    fn updated_item(&mut self, item: Item) -> Vec<AgenticLoopChunk> {
        match item.details {
            ItemDetails::TodoList(list) => vec![notification(
                serde_json::json!({ "kind": "todo_list", "items": list.items }),
                false,
            )],
            other => self.completed_item(Item {
                id: item.id,
                details: other,
            }),
        }
    }

    /// An item finished: the assistant's words, a tool's response
    /// (its call first when it never started), or news.
    fn completed_item(&mut self, item: Item) -> Vec<AgenticLoopChunk> {
        match item.details {
            ItemDetails::AgentMessage(message) => vec![text(message.text)],
            ItemDetails::Reasoning(reasoning) => vec![AgenticLoopChunk::AssistantReasoning(
                AssistantReasoningChunk {
                    r#type: Default::default(),
                    parent_tool_call_id: None,
                    logprobs: None,
                    inner: rmcp::model::TextContent::new(reasoning.text),
                },
            )],
            ItemDetails::CommandExecution(command) => {
                let (id, mut chunks) = self.close(&item.id, "command_execution", || {
                    command_arguments(&command)
                });
                let is_error = matches!(
                    command.status,
                    CommandExecutionStatus::Failed | CommandExecutionStatus::Declined
                );
                let mut result = CallToolResult::success(vec![ContentBlock::text(
                    command.aggregated_output,
                )]);
                result.is_error = Some(is_error);
                chunks.push(response(id, result));
                chunks
            }
            ItemDetails::FileChange(change) => {
                let (id, mut chunks) = self.close(&item.id, "file_change", || {
                    file_change_arguments(&change)
                });
                let lines = change
                    .changes
                    .iter()
                    .map(|change| {
                        let kind = serde_json::to_value(change.kind)
                            .ok()
                            .and_then(|kind| kind.as_str().map(str::to_string))
                            .unwrap_or_default();
                        format!("{kind} {}", change.path)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let mut result = CallToolResult::success(vec![ContentBlock::text(lines)]);
                result.is_error = Some(matches!(change.status, PatchApplyStatus::Failed));
                chunks.push(response(id, result));
                chunks
            }
            ItemDetails::McpToolCall(mcp) => {
                let name = mcp_name(&mcp);
                let (id, mut chunks) =
                    self.close(&item.id, &name, || mcp.arguments.to_string());
                chunks.push(response(id, mcp_result(mcp)));
                chunks
            }
            ItemDetails::WebSearch(search) => {
                let (id, mut chunks) = self.close(&item.id, "web_search", || {
                    web_search_arguments(&search)
                });
                let action = serde_json::to_string(&search.action).unwrap_or_default();
                chunks.push(response(
                    id,
                    CallToolResult::success(vec![ContentBlock::text(action)]),
                ));
                chunks
            }
            ItemDetails::TodoList(list) => vec![notification(
                serde_json::json!({ "kind": "todo_list", "items": list.items }),
                false,
            )],
            ItemDetails::CollabToolCall(collab) => vec![notification(
                serde_json::json!({ "kind": "collab", "call": collab }),
                false,
            )],
            ItemDetails::Error(error) => vec![notification(
                serde_json::json!({ "kind": "codex", "error": error.message }),
                false,
            )],
        }
    }

    /// Mint an id for a starting item.
    fn open(&mut self, item_id: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.calls.insert(item_id.to_string(), id.clone());
        id
    }

    /// The id a completing item answers under: the one minted when it
    /// started, or — never started — a fresh one, with the call it
    /// never had put first.
    fn close(
        &mut self,
        item_id: &str,
        name: &str,
        arguments: impl FnOnce() -> String,
    ) -> (String, Vec<AgenticLoopChunk>) {
        match self.calls.remove(item_id) {
            Some(id) => (id, Vec::new()),
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                let chunks = vec![call(id.clone(), name, arguments())];
                (id, chunks)
            }
        }
    }

    /// The turn's bill: the cumulative count against the baseline.
    ///
    /// `turn.completed` reports the thread's CUMULATIVE usage. The
    /// delta is cumulative minus the last cumulative seen, each
    /// counter clamped at zero; when the cumulative is smaller than
    /// the baseline in total the process is counting from zero, and
    /// the delta is the cumulative itself. Either way the baseline
    /// becomes the cumulative.
    fn usage(&mut self, cumulative: Usage) -> AgenticLoopChunk {
        let total = |usage: &Usage| usage.input_tokens + usage.output_tokens;
        let (prompt, completion) = if total(&cumulative) < total(&self.baseline) {
            (cumulative.input_tokens, cumulative.output_tokens)
        } else {
            (
                (cumulative.input_tokens - self.baseline.input_tokens).max(0),
                (cumulative.output_tokens - self.baseline.output_tokens).max(0),
            )
        };
        self.baseline = cumulative;
        let prompt = prompt.max(0) as u64;
        let completion = completion.max(0) as u64;
        AgenticLoopChunk::Usage(UsageChunk {
            r#type: Default::default(),
            completion_tokens: completion,
            prompt_tokens: prompt,
            total_tokens: prompt + completion,
            meta: None,
        })
    }
}

/// A tool call's name for an MCP call: the tool's, the server being
/// `diverge`; another server is named ahead of it.
fn mcp_name(mcp: &McpToolCall) -> String {
    if mcp.server == crate::config::DIVERGE {
        mcp.tool.clone()
    } else {
        format!("{}/{}", mcp.server, mcp.tool)
    }
}

/// An MCP call's result, as MCP's own: the content blocks parsed
/// into rmcp's, a block that will not parse kept as a text block of
/// its JSON; `structured_content` and `_meta` carried; an error
/// without a result as one text block of its message.
fn mcp_result(mcp: McpToolCall) -> CallToolResult {
    let is_error = matches!(mcp.status, McpToolCallStatus::Failed);
    match mcp.result {
        Some(result) => {
            let content = result
                .content
                .into_iter()
                .map(|block| match serde_json::from_value::<ContentBlock>(block.clone()) {
                    Ok(block) => block,
                    Err(_) => ContentBlock::text(block.to_string()),
                })
                .collect();
            let mut converted = CallToolResult::success(content);
            converted.structured_content = result.structured_content;
            converted.meta = result.meta.and_then(|meta| match meta {
                Value::Object(map) => Some(MetaObject(map)),
                _ => None,
            });
            converted.is_error = Some(is_error);
            converted
        }
        None => {
            let message = mcp
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| "the call returned no result".to_string());
            let mut converted = CallToolResult::success(vec![ContentBlock::text(message)]);
            converted.is_error = Some(true);
            converted
        }
    }
}

/// A command's arguments, as JSON text.
fn command_arguments(command: &CommandExecution) -> String {
    serde_json::json!({ "command": command.command }).to_string()
}

/// A file change's arguments, as JSON text.
fn file_change_arguments(change: &FileChange) -> String {
    serde_json::json!({ "changes": change.changes }).to_string()
}

/// A web search's arguments, as JSON text.
fn web_search_arguments(search: &WebSearch) -> String {
    serde_json::json!({ "query": search.query, "action": search.action }).to_string()
}

/// A tool call chunk, on the main thread.
fn call(id: String, name: &str, arguments: String) -> AgenticLoopChunk {
    AgenticLoopChunk::AssistantToolCall(AssistantToolCallChunk {
        r#type: Default::default(),
        parent_tool_call_id: None,
        id,
        meta: None,
        name: name.to_string(),
        arguments: Some(arguments),
    })
}

/// A tool response chunk, on the main thread.
fn response(id: String, inner: CallToolResult) -> AgenticLoopChunk {
    AgenticLoopChunk::ToolResponse(ToolResponseChunk {
        r#type: Default::default(),
        parent_tool_call_id: None,
        id,
        inner,
    })
}

/// An assistant text chunk, on the main thread.
fn text(text: String) -> AgenticLoopChunk {
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
pub fn notification(message: Value, is_fatal: bool) -> AgenticLoopChunk {
    AgenticLoopChunk::Notification(NotificationChunk {
        r#type: Default::default(),
        is_fatal,
        message,
        meta: None,
    })
}
