import type { FunctionsProfilesComputationsResponseStreamingFunctionExecutionChunk } from "./functionExecutionChunk";
import { functionsExecutionsResponseStreamingTaskChunkMergedList } from "../../../../executions/response/streaming/taskChunkMerged";
import { functionsExecutionsResponseStreamingReasoningSummaryChunkMerged } from "../../../../executions/response/streaming/reasoningSummaryChunkMerged";
import { agentCompletionsResponseUsageMerged } from "../../../../../agent/completions/response/usageMerged";

export function functionsProfilesComputationsResponseStreamingFunctionExecutionChunkMerged(
  a: FunctionsProfilesComputationsResponseStreamingFunctionExecutionChunk,
  b: FunctionsProfilesComputationsResponseStreamingFunctionExecutionChunk,
): [FunctionsProfilesComputationsResponseStreamingFunctionExecutionChunk, boolean] {
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
    dataset: a.dataset,
    n: a.n,
    retry: a.retry,
    id: a.id,
    tasks,
    ...(tasks_errors != null ? { tasks_errors } : {}),
    ...(reasoning != null ? { reasoning } : {}),
    ...(output != null ? { output } : {}),
    ...(error != null ? { error } : {}),
    created: a.created,
    function: a.function,
    profile: a.profile,
    object: a.object,
    ...(usage != null ? { usage } : {}),
  }, true];
}

export function functionsProfilesComputationsResponseStreamingFunctionExecutionChunkMergedList(
  a: FunctionsProfilesComputationsResponseStreamingFunctionExecutionChunk[],
  b: FunctionsProfilesComputationsResponseStreamingFunctionExecutionChunk[],
): [FunctionsProfilesComputationsResponseStreamingFunctionExecutionChunk[], boolean] {
  let changed = false;
  const result = [...a];
  for (const bItem of b) {
    const existingIdx = result.findIndex((x) => x.index === bItem.index);
    if (existingIdx !== -1) {
      const [merged, c] = functionsProfilesComputationsResponseStreamingFunctionExecutionChunkMerged(result[existingIdx], bItem);
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
