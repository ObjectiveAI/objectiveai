import type { FunctionsExecutionsResponseStreamingReasoningSummaryChunk } from "./reasoningSummaryChunk";
import { agentCompletionsResponseStreamingMessageChunkMergedList } from "../../../../agent/completions/response/streaming/messageChunkMerged";
import { agentCompletionsResponseUsageMerged } from "../../../../agent/completions/response/usageMerged";

export function functionsExecutionsResponseStreamingReasoningSummaryChunkMerged(
  a: FunctionsExecutionsResponseStreamingReasoningSummaryChunk,
  b: FunctionsExecutionsResponseStreamingReasoningSummaryChunk,
): [FunctionsExecutionsResponseStreamingReasoningSummaryChunk, boolean] {
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

  if (!changed) return [a, false];
  return [{
    id: a.id,
    agent_full_id: a.agent_full_id,
    agent_id: a.agent_id,
    agent_instance_hierarchy: a.agent_instance_hierarchy,
    ...(a.agent_remote != null ? { agent_remote: a.agent_remote } : {}),
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
