//! Delta type for streaming chat completion responses.

use crate::chat::completions::response;
use serde::{Deserialize, Serialize};

/// A delta (incremental update) in a streaming response.
///
/// Each field contains only the new content since the last delta.
/// Deltas can be accumulated using the [`push`](Self::push) method.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Delta {
    /// New content text since the last delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// New refusal text since the last delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    /// The role (only present in the first delta).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<response::Role>,
    /// Tool call updates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<super::ToolCall>>,

    /// New reasoning text since the last delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// New generated images.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<response::Image>>,
}

impl Delta {
    pub fn push(
        &mut self,
        Delta {
            content,
            refusal,
            role,
            tool_calls,
            reasoning,
            images,
        }: &Delta,
    ) {
        response::util::push_option_string(&mut self.content, content);
        response::util::push_option_string(&mut self.refusal, refusal);
        if self.role.is_none() {
            self.role = role.clone();
        }
        self.push_tool_calls(tool_calls);
        response::util::push_option_string(&mut self.reasoning, reasoning);
        response::util::push_option_vec(&mut self.images, images);
    }

    fn push_tool_calls(
        &mut self,
        other_tool_calls: &Option<Vec<super::ToolCall>>,
    ) {
        fn push_tool_call(
            tool_calls: &mut Vec<super::ToolCall>,
            other: &super::ToolCall,
        ) {
            fn find_tool_call(
                tool_calls: &mut Vec<super::ToolCall>,
                index: u64,
            ) -> Option<&mut super::ToolCall> {
                for tool_call in tool_calls {
                    if tool_call.index == index {
                        return Some(tool_call);
                    }
                }
                None
            }
            if let Some(tool_call) = find_tool_call(tool_calls, other.index) {
                tool_call.push(other);
            } else {
                tool_calls.push(other.clone());
            }
        }
        match (self.tool_calls.as_mut(), other_tool_calls) {
            (Some(self_tool_calls), Some(other_tool_calls)) => {
                for other_tool_call in other_tool_calls {
                    push_tool_call(self_tool_calls, other_tool_call);
                }
            }
            (None, Some(other_tool_calls)) => {
                self.tool_calls = Some(other_tool_calls.clone());
            }
            _ => {}
        }
    }
}
