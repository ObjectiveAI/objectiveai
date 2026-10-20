// Joining the streamed pieces of one chunk.

import type { AgenticLoopChunk } from "./chunk.js";
import type { AssistantToolCallChunk } from "./assistant_tool_call_chunk.js";

/**
 * Push one chunk onto a list of them, joining what the loop streams
 * in pieces: reasoning, text and a refusal append their text, and
 * their logprobs, to the last chunk of the same kind under the same
 * tool call; a tool call appends its `arguments` to the call of the
 * same `id` under the same parent, wherever it is; everything else
 * is one more chunk. The list reads as the loop meant it, however
 * the pieces came.
 */
export function push(chunks: AgenticLoopChunk[], chunk: AgenticLoopChunk): void {
  const last = chunks[chunks.length - 1];
  switch (chunk.type) {
    case "assistant_reasoning":
    case "assistant_text_content":
    case "assistant_refusal":
      if (last !== undefined && last.type === chunk.type && last.parent_tool_call_id === chunk.parent_tool_call_id) {
        last.text += chunk.text;
        if (chunk.logprobs !== undefined) {
          last.logprobs = last.logprobs === undefined ? chunk.logprobs : [...last.logprobs, ...chunk.logprobs];
        }
        return;
      }
      break;
    case "assistant_tool_call":
      pushToolCall(chunks, chunk);
      return;
    default:
      break;
  }
  chunks.push(chunk);
}

/** A tool call's piece onto the call it belongs to, or on the end. */
function pushToolCall(chunks: AgenticLoopChunk[], chunk: AssistantToolCallChunk): void {
  for (let at = chunks.length - 1; at >= 0; at -= 1) {
    const existing = chunks[at];
    if (existing !== undefined && existing.type === "assistant_tool_call" && existing.id === chunk.id && existing.parent_tool_call_id === chunk.parent_tool_call_id) {
      if (chunk.arguments !== undefined) {
        existing.arguments = existing.arguments === undefined ? chunk.arguments : existing.arguments + chunk.arguments;
      }
      return;
    }
  }
  chunks.push(chunk);
}
