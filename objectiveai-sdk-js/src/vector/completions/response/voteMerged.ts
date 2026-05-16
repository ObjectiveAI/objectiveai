import type { VectorCompletionsResponseVote } from "./vote";

export function vectorCompletionsResponseVoteMergedList(
  a: VectorCompletionsResponseVote[],
  b: VectorCompletionsResponseVote[],
): [VectorCompletionsResponseVote[], boolean] {
  if (b.length === 0) return [a, false];
  return [[...a, ...b], true];
}
