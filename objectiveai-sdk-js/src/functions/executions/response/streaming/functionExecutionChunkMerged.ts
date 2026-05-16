import type { FunctionsExecutionsResponseStreamingFunctionExecutionChunk } from "./functionExecutionChunk";
import { functionsExecutionsResponseStreamingTaskChunkMergedList } from "./taskChunkMerged";
import { functionsExecutionsResponseStreamingReasoningSummaryChunkMerged } from "./reasoningSummaryChunkMerged";
import { agentCompletionsResponseUsageMerged } from "../../../../agent/completions/response/usageMerged";

export function functionsExecutionsResponseStreamingFunctionExecutionChunkMerged(
  a: FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  b: FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
): [FunctionsExecutionsResponseStreamingFunctionExecutionChunk, boolean] {
  let changed = false;

  const [tasks, c1] = functionsExecutionsResponseStreamingTaskChunkMergedList(a.tasks, b.tasks);
  if (c1) changed = true;

  let tasks_errors = a.tasks_errors;
  if (b.tasks_errors === true) {
    if (a.tasks_errors !== true) changed = true;
    tasks_errors = true;
  }

  let reasoning = a.reasoning;
  if (a.reasoning != null && b.reasoning != null) {
    const [merged, c] = functionsExecutionsResponseStreamingReasoningSummaryChunkMerged(a.reasoning, b.reasoning);
    reasoning = merged;
    if (c) changed = true;
  } else if (b.reasoning != null) {
    reasoning = b.reasoning;
    changed = true;
  }

  let output = a.output;
  if (b.output != null) {
    if (b.output !== a.output) changed = true;
    output = b.output;
  }

  let retry_token = a.retry_token;
  if (b.retry_token != null) {
    if (b.retry_token !== a.retry_token) changed = true;
    retry_token = b.retry_token;
  }

  let error = a.error;
  if (b.error != null) {
    if (b.error !== a.error) changed = true;
    error = b.error;
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

  if (!changed) return [a, false];
  return [{
    id: a.id,
    tasks,
    ...(tasks_errors != null ? { tasks_errors } : {}),
    ...(reasoning != null ? { reasoning } : {}),
    ...(output != null ? { output } : {}),
    ...(error != null ? { error } : {}),
    ...(retry_token != null ? { retry_token } : {}),
    created: a.created,
    ...(a.function !== undefined ? { function: a.function } : {}),
    ...(a.profile !== undefined ? { profile: a.profile } : {}),
    object: a.object,
    ...(usage != null ? { usage } : {}),
  }, true];
}
