import type { FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk } from "./functionInventionRecursiveChunk";
import { functionsInventionsRecursiveResponseStreamingFunctionInventionChunkMergedList } from "./functionInventionChunkMerged";
import { agentCompletionsResponseUsageMerged } from "../../../../../agent/completions/response/usageMerged";

export function functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged(
  a: FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk,
  b: FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk,
): [FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk, boolean] {
  let changed = false;

  const [inventions, c1] = functionsInventionsRecursiveResponseStreamingFunctionInventionChunkMergedList(a.inventions, b.inventions);
  if (c1) changed = true;

  let inventions_errors = a.inventions_errors;
  if (b.inventions_errors === true) {
    if (a.inventions_errors !== true) changed = true;
    inventions_errors = true;
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
    inventions,
    ...(inventions_errors != null ? { inventions_errors } : {}),
    created: a.created,
    object: a.object,
    ...(usage != null ? { usage } : {}),
  }, true];
}
