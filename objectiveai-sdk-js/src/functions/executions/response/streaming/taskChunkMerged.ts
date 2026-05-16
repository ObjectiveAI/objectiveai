import type { FunctionsExecutionsResponseStreamingTaskChunk } from "./taskChunk";
import type { FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk } from "./functionExecutionTaskChunk";
import type { FunctionsExecutionsResponseStreamingVectorCompletionTaskChunk } from "./vectorCompletionTaskChunk";
import { functionsExecutionsResponseStreamingVectorCompletionTaskChunkMerged } from "./vectorCompletionTaskChunkMerged";
import { functionsExecutionsResponseStreamingReasoningSummaryChunkMerged } from "./reasoningSummaryChunkMerged";
import { agentCompletionsResponseUsageMerged } from "../../../../agent/completions/response/usageMerged";

function isVectorCompletionTaskChunk(
  chunk: FunctionsExecutionsResponseStreamingTaskChunk,
): chunk is FunctionsExecutionsResponseStreamingVectorCompletionTaskChunk {
  return "scores" in chunk;
}

function taskChunkIndex(chunk: FunctionsExecutionsResponseStreamingTaskChunk): number {
  return (chunk as { index: number }).index;
}

function functionsExecutionsResponseStreamingFunctionExecutionTaskChunkMerged(
  a: FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk,
  b: FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk,
): [FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk, boolean] {
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
    index: a.index,
    task_index: a.task_index,
    task_path: a.task_path,
    ...(a.swiss_pool_index != null ? { swiss_pool_index: a.swiss_pool_index } : {}),
    ...(a.swiss_round != null ? { swiss_round: a.swiss_round } : {}),
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

export function functionsExecutionsResponseStreamingTaskChunkMerged(
  a: FunctionsExecutionsResponseStreamingTaskChunk,
  b: FunctionsExecutionsResponseStreamingTaskChunk,
): [FunctionsExecutionsResponseStreamingTaskChunk, boolean] {
  if (isVectorCompletionTaskChunk(a) && isVectorCompletionTaskChunk(b)) {
    return functionsExecutionsResponseStreamingVectorCompletionTaskChunkMerged(a, b);
  }
  if (!isVectorCompletionTaskChunk(a) && !isVectorCompletionTaskChunk(b)) {
    return functionsExecutionsResponseStreamingFunctionExecutionTaskChunkMerged(a, b);
  }
  return [a, false];
}

export function functionsExecutionsResponseStreamingTaskChunkMergedList(
  a: FunctionsExecutionsResponseStreamingTaskChunk[],
  b: FunctionsExecutionsResponseStreamingTaskChunk[],
): [FunctionsExecutionsResponseStreamingTaskChunk[], boolean] {
  let changed = false;
  const result = [...a];
  for (const bItem of b) {
    const bIndex = taskChunkIndex(bItem);
    const existingIdx = result.findIndex((x) => taskChunkIndex(x) === bIndex);
    if (existingIdx !== -1) {
      const [merged, c] = functionsExecutionsResponseStreamingTaskChunkMerged(result[existingIdx], bItem);
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
