import type { FunctionsInventionsResponseStreamingFunctionInventionChunk } from "./functionInventionChunk";
import { functionsInventionsResponseStreamingAgentCompletionChunkMergedList } from "./agentCompletionChunkMerged";
import { agentCompletionsResponseUsageMerged } from "../../../../agent/completions/response/usageMerged";

export function functionsInventionsResponseStreamingFunctionInventionChunkMerged(
  a: FunctionsInventionsResponseStreamingFunctionInventionChunk,
  b: FunctionsInventionsResponseStreamingFunctionInventionChunk,
): [FunctionsInventionsResponseStreamingFunctionInventionChunk, boolean] {
  let changed = false;

  const [completions, c1] = functionsInventionsResponseStreamingAgentCompletionChunkMergedList(a.completions, b.completions);
  if (c1) changed = true;

  let state = a.state;
  if (b.state != null) {
    if (b.state !== a.state) changed = true;
    state = b.state;
  }

  let path = a.path;
  if (b.path != null) {
    if (b.path !== a.path) changed = true;
    path = b.path;
  }

  let fn = a.function;
  if (b.function != null) {
    if (b.function !== a.function) changed = true;
    fn = b.function;
  }

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

  if (!changed) return [a, false];
  return [{
    id: a.id,
    completions,
    ...(state != null ? { state } : {}),
    ...(path != null ? { path } : {}),
    ...(fn != null ? { function: fn } : {}),
    created: a.created,
    object: a.object,
    ...(usage != null ? { usage } : {}),
    ...(error != null ? { error } : {}),
  }, true];
}
