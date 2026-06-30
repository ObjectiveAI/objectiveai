"use client";

import {
  vectorCompletionsCreateVectorCompletion,
  vectorCompletionsResponseStreamingVectorCompletionChunkMerged,
} from "@objectiveai/sdk";
import type {
  VectorCompletionsResponseStreamingVectorCompletionChunk,
} from "@objectiveai/sdk";
import { getClient } from "./sdk";
import { useSDKStream } from "./useSDKStream";
import type { StreamState } from "./useSDKStream";

type Chunk = VectorCompletionsResponseStreamingVectorCompletionChunk;

export type VectorCompletionState = StreamState;

export interface DisplayVote {
  model: string;
  vote: number[];
  weight: number;
}

export interface VectorCompletionResult {
  chunk: Chunk | null;
  votes: DisplayVote[];
  scores: number[];
  weights: number[];
  state: VectorCompletionState;
  error: string | null;
  run: () => void;
  abort: () => void;
}

interface UseVectorCompletionParams {
  prompt: string;
  responses: string[];
  swarmOwner: string;
  swarmRepo: string;
  swarmCommit?: string | null;
}

export function useVectorCompletion({
  prompt,
  responses,
  swarmOwner,
  swarmRepo,
  swarmCommit,
}: UseVectorCompletionParams): VectorCompletionResult {
  const { chunk, state, error, start: run, abort } = useSDKStream<Chunk>({
    createStream: (signal) => {
      const client = getClient();
      return vectorCompletionsCreateVectorCompletion(
        client,
        {
          messages: [{ role: "user", content: prompt }],
          responses: responses.map((r) => r),
          swarm: {
            remote: "github" as const,
            owner: swarmOwner,
            repository: swarmRepo,
            ...(swarmCommit ? { commit: swarmCommit } : {}),
          },
          stream: true as const,
        },
        { signal },
      );
    },
    merge: (acc, next) => vectorCompletionsResponseStreamingVectorCompletionChunkMerged(acc, next)[0],
    deps: [prompt, responses, swarmOwner, swarmRepo, swarmCommit],
  });

  const { votes, scores, weights } = extractData(chunk);

  return { chunk, votes, scores, weights, state, error, run, abort };
}

function extractData(chunk: Chunk | null): {
  votes: DisplayVote[];
  scores: number[];
  weights: number[];
} {
  if (!chunk) return { votes: [], scores: [], weights: [] };

  const votes: DisplayVote[] = (chunk.votes ?? []).map((v) => ({
    model: v.agent_full_id,
    vote: v.vote,
    weight: v.weight,
  }));

  return {
    votes,
    scores: chunk.scores ?? [],
    weights: chunk.weights ?? [],
  };
}
