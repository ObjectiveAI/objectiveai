import type { VectorCompletionsResponseStreamingVectorCompletionChunk } from "./vectorCompletionChunk";
import { vectorCompletionsResponseStreamingAgentCompletionChunkMergedList } from "./agentCompletionChunkMerged";
import { vectorCompletionsResponseVoteMergedList } from "../voteMerged";
import { mergedNumberArray } from "../../../../merge";
import { agentCompletionsResponseUsageMerged } from "../../../../agent/completions/response/usageMerged";

export function vectorCompletionsResponseStreamingVectorCompletionChunkMerged(
  a: VectorCompletionsResponseStreamingVectorCompletionChunk,
  b: VectorCompletionsResponseStreamingVectorCompletionChunk,
): [VectorCompletionsResponseStreamingVectorCompletionChunk, boolean] {
  let changed = false;

  const [completions, c1] = vectorCompletionsResponseStreamingAgentCompletionChunkMergedList(a.completions, b.completions);
  if (c1) changed = true;

  const [votes, c2] = vectorCompletionsResponseVoteMergedList(a.votes, b.votes);
  if (c2) changed = true;

  const [scores, c3] = mergedNumberArray(a.scores, b.scores);
  if (c3) changed = true;

  const [weights, c4] = mergedNumberArray(a.weights, b.weights);
  if (c4) changed = true;

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
    completions,
    votes,
    scores,
    weights,
    created: a.created,
    swarm: a.swarm,
    object: a.object,
    ...(usage != null ? { usage } : {}),
  }, true];
}
