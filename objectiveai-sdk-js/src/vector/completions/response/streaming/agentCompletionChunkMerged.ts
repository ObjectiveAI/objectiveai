import { merge } from "../../../../merge";
import type { VectorCompletionsResponseStreamingAgentCompletionChunk } from "./agentCompletionChunk";
import { agentCompletionsResponseStreamingMessageChunkMergedList } from "../../../../agent/completions/response/streaming/messageChunkMerged";
import { agentCompletionsResponseUsageMerged } from "../../../../agent/completions/response/usageMerged";

export function vectorCompletionsResponseStreamingAgentCompletionChunkMerged(
  a: VectorCompletionsResponseStreamingAgentCompletionChunk,
  b: VectorCompletionsResponseStreamingAgentCompletionChunk,
): [VectorCompletionsResponseStreamingAgentCompletionChunk, boolean] {
  let changed = false;

  const [messages, c1] = agentCompletionsResponseStreamingMessageChunkMergedList(a.messages, b.messages);
  if (c1) changed = true;

  let usage = a.usage;
  if (a.usage != null && b.usage != null) {
    const [merged, c] = agentCompletionsResponseUsageMerged(a.usage, b.usage);
    usage = merged;
    if (c) changed = true;
  } else if (b.usage != null) {
    usage = b.usage;
    changed = true;
  }

  let error = a.error;
  if (b.error != null) {
    if (b.error !== a.error) changed = true;
    error = b.error;
  }

  let continuation = a.continuation;
  if (b.continuation != null) {
    if (b.continuation !== a.continuation) changed = true;
    continuation = b.continuation;
  }

  // messages_queued: replace (latest non-null wins)
  let messages_queued = a.messages_queued;
  if (b.messages_queued != null) {
    if (b.messages_queued !== a.messages_queued) changed = true;
    messages_queued = b.messages_queued;
  }

  // First chunk wins: agent_inline rides only the completion's first
  // chunk, so the accumulator never overwrites it.
  let agent_inline = a.agent_inline;
  if (agent_inline == null && b.agent_inline != null) {
    agent_inline = b.agent_inline;
    changed = true;
  }

  // First chunk wins: request_choice_keys ride only the completion's
  // first chunk, so the accumulator never overwrites them.
  let request_choice_keys = a.request_choice_keys;
  if (request_choice_keys == null && b.request_choice_keys != null) {
    request_choice_keys = b.request_choice_keys;
    changed = true;
  }

  if (!changed) return [a, false];
  return [{
    index: a.index,
    id: a.id,
    agent_full_id: a.agent_full_id,
    agent_id: a.agent_id,
    agent_instance_hierarchy: a.agent_instance_hierarchy,
    ...(a.agent_remote != null ? { agent_remote: a.agent_remote } : {}),
    ...(agent_inline != null ? { agent_inline } : {}),
    ...(request_choice_keys != null ? { request_choice_keys } : {}),
    created: a.created,
    messages,
    object: a.object,
    ...(usage != null ? { usage } : {}),
    upstream: a.upstream,
    ...(error != null ? { error } : {}),
    ...(continuation != null ? { continuation } : {}),
    ...(messages_queued != null ? { messages_queued } : {}),
  }, true];
}

export function vectorCompletionsResponseStreamingAgentCompletionChunkMergedList(
  a: VectorCompletionsResponseStreamingAgentCompletionChunk[],
  b: VectorCompletionsResponseStreamingAgentCompletionChunk[],
): [VectorCompletionsResponseStreamingAgentCompletionChunk[], boolean] {
  let changed = false;
  const result = [...a];
  for (const bItem of b) {
    const existingIdx = result.findIndex((x) => x.index === bItem.index);
    if (existingIdx !== -1) {
      const [merged, c] = vectorCompletionsResponseStreamingAgentCompletionChunkMerged(result[existingIdx], bItem);
      if (c) {
        result[existingIdx] = merged;
        changed = true;
      }
    } else {
      result.push(bItem);
      changed = true;
    }
  }
  if (!changed) return [a, false];
  return [result, true];
}
