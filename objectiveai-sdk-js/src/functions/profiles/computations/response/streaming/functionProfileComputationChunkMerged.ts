import type { FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk } from "./functionProfileComputationChunk";
import { functionsProfilesComputationsResponseStreamingFunctionExecutionChunkMergedList } from "./functionExecutionChunkMerged";
import { agentCompletionsResponseUsageMerged } from "../../../../../agent/completions/response/usageMerged";

export function functionsProfilesComputationsResponseStreamingFunctionProfileComputationChunkMerged(
  a: FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk,
  b: FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk,
): [FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk, boolean] {
  let changed = false;

  const [executions, c1] = functionsProfilesComputationsResponseStreamingFunctionExecutionChunkMergedList(a.executions, b.executions);
  if (c1) changed = true;

  let executions_errors = a.executions_errors;
  if (b.executions_errors === true) {
    if (a.executions_errors !== true) changed = true;
    executions_errors = true;
  }

  let profile = a.profile;
  if (b.profile != null) {
    if (b.profile !== a.profile) changed = true;
    profile = b.profile;
  }

  let fitting_stats = a.fitting_stats;
  if (b.fitting_stats != null) {
    if (b.fitting_stats !== a.fitting_stats) changed = true;
    fitting_stats = b.fitting_stats;
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
    ...a,
    executions,
    ...(executions_errors != null ? { executions_errors } : {}),
    ...(profile != null ? { profile } : {}),
    ...(fitting_stats != null ? { fitting_stats } : {}),
    ...(usage != null ? { usage } : {}),
  }, true];
}
