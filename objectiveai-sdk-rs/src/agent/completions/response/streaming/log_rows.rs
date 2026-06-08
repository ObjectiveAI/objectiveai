//! Streaming-content `log_rows` walkers for agent-completion chunks.
//!
//! Three boxed-iterator entry points form a recursive walk over the
//! chunk tree. Each yields one [`LogValue`] at a time; nothing
//! collects ahead. Box is used only at the per-message dispatch
//! point (assistant vs tool branch type erasure) — at most one
//! allocation per message, never per leaf row.

use crate::agent::completions::message::{RichContent, RichContentPart};
use crate::agent::completions::response::ToolResponse;
use crate::logs::{LogRowIter, LogValue};

use super::{AgentCompletionChunk, AssistantResponseChunk, MessageChunk};

impl AgentCompletionChunk {
    /// Iterate over every streaming-content row this chunk implies,
    /// in writer-feed order. Never collects: each yielded
    /// [`LogValue`] borrows from `self`.
    pub fn log_rows<'a>(&'a self) -> LogRowIter<'a> {
        let response_id = self.id.as_str();
        Box::new(
            self.messages
                .iter()
                .flat_map(move |msg| message_chunk_log_rows(response_id, msg)),
        )
    }
}

pub(crate) fn message_chunk_log_rows<'a>(
    response_id: &'a str,
    msg: &'a MessageChunk,
) -> LogRowIter<'a> {
    match msg {
        MessageChunk::Assistant(a) => assistant_response_chunk_log_rows(response_id, a),
        MessageChunk::Tool(t) => tool_response_log_rows(response_id, t),
    }
}

fn assistant_response_chunk_log_rows<'a>(
    response_id: &'a str,
    chunk: &'a AssistantResponseChunk,
) -> LogRowIter<'a> {
    let index = chunk.index;

    let refusal_iter = chunk.refusal.iter().map(move |text| {
        LogValue::AssistantResponseRefusal { response_id, index, text: text.as_str() }
    });

    let reasoning_iter = chunk.reasoning.iter().map(move |text| {
        LogValue::AssistantResponseReasoning { response_id, index, text: text.as_str() }
    });

    let tool_calls_iter = chunk
        .tool_calls
        .iter()
        .flatten()
        .enumerate()
        .filter_map(move |(tc_idx, tc)| {
            let id = tc.id.as_deref()?;
            let args = tc.function.as_ref().and_then(|f| f.arguments.as_deref())?;
            Some(LogValue::AssistantResponseToolCalls {
                response_id,
                index,
                tool_call_index: tc_idx as u64,
                tool_call_id: id,
                arguments: args,
            })
        });

    let content_iter = chunk
        .content
        .iter()
        .flat_map(move |content| assistant_content_log_rows(response_id, index, content));

    Box::new(
        refusal_iter
            .chain(reasoning_iter)
            .chain(tool_calls_iter)
            .chain(content_iter),
    )
}

fn tool_response_log_rows<'a>(
    response_id: &'a str,
    response: &'a ToolResponse,
) -> LogRowIter<'a> {
    let index = response.index;
    let head = std::iter::once(LogValue::ToolResponse {
        response_id,
        index,
        tool_call_id: response.inner.tool_call_id.as_str(),
    });
    let content_iter = tool_content_log_rows(response_id, index, &response.inner.content);
    Box::new(head.chain(content_iter))
}

fn assistant_content_log_rows<'a>(
    response_id: &'a str,
    index: u64,
    content: &'a RichContent,
) -> LogRowIter<'a> {
    match content {
        RichContent::Text(text) => Box::new(std::iter::once(LogValue::AssistantResponseContentText {
            response_id,
            index,
            part_index: 0,
            text: text.as_str(),
        })),
        RichContent::Parts(parts) => Box::new(parts.iter().enumerate().map(move |(part_index, part)| {
            assistant_content_part(response_id, index, part_index as u64, part)
        })),
    }
}

fn assistant_content_part<'a>(
    response_id: &'a str,
    index: u64,
    part_index: u64,
    part: &'a RichContentPart,
) -> LogValue<'a> {
    match part {
        RichContentPart::Text { text } => LogValue::AssistantResponseContentText {
            response_id,
            index,
            part_index,
            text: text.as_str(),
        },
        RichContentPart::ImageUrl { image_url } => LogValue::AssistantResponseContentImage {
            response_id,
            index,
            part_index,
            image_url,
        },
        RichContentPart::InputAudio { input_audio } => LogValue::AssistantResponseContentAudio {
            response_id,
            index,
            part_index,
            input_audio,
        },
        RichContentPart::InputVideo { video_url } => LogValue::AssistantResponseContentVideo {
            response_id,
            index,
            part_index,
            video_url,
            is_input: true,
        },
        RichContentPart::VideoUrl { video_url } => LogValue::AssistantResponseContentVideo {
            response_id,
            index,
            part_index,
            video_url,
            is_input: false,
        },
        RichContentPart::File { file } => LogValue::AssistantResponseContentFile {
            response_id,
            index,
            part_index,
            file,
        },
    }
}

fn tool_content_log_rows<'a>(
    response_id: &'a str,
    index: u64,
    content: &'a RichContent,
) -> LogRowIter<'a> {
    match content {
        RichContent::Text(text) => Box::new(std::iter::once(LogValue::ToolResponseContentText {
            response_id,
            index,
            part_index: 0,
            text: text.as_str(),
        })),
        RichContent::Parts(parts) => Box::new(parts.iter().enumerate().map(move |(part_index, part)| {
            tool_content_part(response_id, index, part_index as u64, part)
        })),
    }
}

fn tool_content_part<'a>(
    response_id: &'a str,
    index: u64,
    part_index: u64,
    part: &'a RichContentPart,
) -> LogValue<'a> {
    match part {
        RichContentPart::Text { text } => LogValue::ToolResponseContentText {
            response_id,
            index,
            part_index,
            text: text.as_str(),
        },
        RichContentPart::ImageUrl { image_url } => LogValue::ToolResponseContentImage {
            response_id,
            index,
            part_index,
            image_url,
        },
        RichContentPart::InputAudio { input_audio } => LogValue::ToolResponseContentAudio {
            response_id,
            index,
            part_index,
            input_audio,
        },
        RichContentPart::InputVideo { video_url } => LogValue::ToolResponseContentVideo {
            response_id,
            index,
            part_index,
            video_url,
            is_input: true,
        },
        RichContentPart::VideoUrl { video_url } => LogValue::ToolResponseContentVideo {
            response_id,
            index,
            part_index,
            video_url,
            is_input: false,
        },
        RichContentPart::File { file } => LogValue::ToolResponseContentFile {
            response_id,
            index,
            part_index,
            file,
        },
    }
}
