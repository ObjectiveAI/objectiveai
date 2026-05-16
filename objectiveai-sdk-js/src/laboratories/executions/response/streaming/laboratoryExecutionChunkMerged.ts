import type { LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk } from "./laboratoryExecutionChunk";
import { laboratoriesExecutionsResponseStreamingBuilderChunkMergedList } from "./builderChunkMerged";
import { laboratoriesExecutionsResponseStreamingEvaluationChunkMergedList } from "./evaluationChunkMerged";
import { agentCompletionsResponseUsageMerged } from "../../../../agent/completions/response/usageMerged";

export function laboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged(
  a: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk,
  b: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk,
): [LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk, boolean] {
  let changed = false;

  const [builders, c1] = laboratoriesExecutionsResponseStreamingBuilderChunkMergedList(a.builders, b.builders);
  if (c1) changed = true;

  const [evaluations, c2] = laboratoriesExecutionsResponseStreamingEvaluationChunkMergedList(a.evaluations, b.evaluations);
  if (c2) changed = true;

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
    builders,
    evaluations,
    created: a.created,
    object: a.object,
    ...(error != null ? { error } : {}),
    ...(usage != null ? { usage } : {}),
  }, true];
}
