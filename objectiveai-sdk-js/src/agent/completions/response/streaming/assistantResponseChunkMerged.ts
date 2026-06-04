import { merge, mergedString } from "../../../../merge";
import type { AgentCompletionsResponseStreamingAssistantResponseChunk } from "./assistantResponseChunk";
import { agentCompletionsMessageAssistantToolCallDeltaMergedList } from "../../message/assistantToolCallDeltaMerged";
import { agentCompletionsMessageRichContentMerged } from "../../message/richContentMerged";
import { agentCompletionsResponseLogprobsMerged } from "../logprobsMerged";
import { agentCompletionsResponseUpstreamUsageMerged } from "../upstreamUsageMerged";

function mergedOptionString(
  a: string | null | undefined,
  b: string | null | undefined,
): [string | null | undefined, boolean] {
  if (a != null && b != null) {
    return mergedString(a, b);
  } else if (b != null) {
    return [b, true];
  }
  return [a, false];
}

export function agentCompletionsResponseStreamingAssistantResponseChunkMerged(
  a: AgentCompletionsResponseStreamingAssistantResponseChunk,
  b: AgentCompletionsResponseStreamingAssistantResponseChunk,
): [AgentCompletionsResponseStreamingAssistantResponseChunk, boolean] {
  let changed = false;

  const [reasoning, c1] = mergedOptionString(a.reasoning, b.reasoning);
  if (c1) changed = true;

  let tool_calls = a.tool_calls;
  if (a.tool_calls != null && b.tool_calls != null) {
    const [merged, c] = agentCompletionsMessageAssistantToolCallDeltaMergedList(a.tool_calls, b.tool_calls);
    tool_calls = merged;
    if (c) changed = true;
  } else if (b.tool_calls != null) {
    tool_calls = b.tool_calls;
    changed = true;
  }

  let content = a.content;
  if (a.content != null && b.content != null) {
    const [merged, c] = agentCompletionsMessageRichContentMerged(a.content, b.content);
    content = merged;
    if (c) changed = true;
  } else if (b.content != null) {
    content = b.content;
    changed = true;
  }

  const [refusal, c2] = mergedOptionString(a.refusal, b.refusal);
  if (c2) changed = true;

  let finish_reason = a.finish_reason;
  if (a.finish_reason == null && b.finish_reason != null) {
    finish_reason = b.finish_reason;
    changed = true;
  }

  let logprobs = a.logprobs;
  if (a.logprobs != null && b.logprobs != null) {
    const [merged, c] = agentCompletionsResponseLogprobsMerged(a.logprobs, b.logprobs);
    logprobs = merged;
    if (c) changed = true;
  } else if (b.logprobs != null) {
    logprobs = b.logprobs;
    changed = true;
  }

  let upstream_id = a.upstream_id;
  if (a.upstream_id === "" && b.upstream_id !== "") {
    upstream_id = b.upstream_id;
    changed = true;
  }

  let service_tier = a.service_tier;
  if (a.service_tier == null && b.service_tier != null) {
    service_tier = b.service_tier;
    changed = true;
  }

  let system_fingerprint = a.system_fingerprint;
  if (a.system_fingerprint == null && b.system_fingerprint != null) {
    system_fingerprint = b.system_fingerprint;
    changed = true;
  }

  let provider = a.provider;
  if (a.provider == null && b.provider != null) {
    provider = b.provider;
    changed = true;
  }

  let usage = a.usage;
  if (a.usage != null && b.usage != null) {
    const [merged, c] = agentCompletionsResponseUpstreamUsageMerged(a.usage, b.usage);
    usage = merged;
    if (c) changed = true;
  } else if (b.usage != null) {
    usage = b.usage;
    changed = true;
  }

  if (!changed) return [a, false];
  return [{
    role: a.role,
    index: a.index,
    created: a.created,
    model: a.model,
    upstream_id,
    ...(reasoning != null ? { reasoning } : {}),
    ...(tool_calls != null ? { tool_calls } : {}),
    ...(content != null ? { content } : {}),
    ...(refusal != null ? { refusal } : {}),
    // finish_reason: no skip_serializing_if — must be present (null or value)
    ...(finish_reason !== undefined ? { finish_reason } : {}),
    ...(logprobs != null ? { logprobs } : {}),
    ...(service_tier != null ? { service_tier } : {}),
    ...(system_fingerprint != null ? { system_fingerprint } : {}),
    ...(provider != null ? { provider } : {}),
    ...(usage != null ? { usage } : {}),
  }, true];
}
