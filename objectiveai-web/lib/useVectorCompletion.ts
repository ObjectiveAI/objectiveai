"use client";

import { useState, useRef, useCallback } from "react";
import {
  vectorCompletionsCreateVectorCompletion,
  vectorCompletionsResponseStreamingVectorCompletionChunkMerged,
} from "objectiveai";
import type {
  VectorCompletionsResponseStreamingVectorCompletionChunk,
} from "objectiveai";
import { getClient } from "./sdk";

type Chunk = VectorCompletionsResponseStreamingVectorCompletionChunk;

export type VectorCompletionState = "idle" | "streaming" | "done" | "error";

export interface DisplayVote {
  model: string;
  vote: number[];
  weight: number;
}

export interface VectorCompletionResult {
  /** Raw accumulated chunk */
  chunk: Chunk | null;
  /** Votes extracted from chunk */
  votes: DisplayVote[];
  /** Current scores vector */
  scores: number[];
  /** Current weights vector */
  weights: number[];
  /** Lifecycle state */
  state: VectorCompletionState;
  /** Error message */
  error: string | null;
  /** Start completion */
  run: () => void;
  /** Abort */
  abort: () => void;
}

interface UseVectorCompletionParams {
  /** Prompt messages */
  prompt: string;
  /** Response options to vote on */
  responses: string[];
  /** Swarm reference */
  swarmOwner: string;
  swarmRepo: string;
  swarmCommit?: string | null;
}

/**
 * Hook for streaming a vector completion via the ObjectiveAI SDK.
 *
 * Simplest hook — single task, no function composition. Agents vote on
 * responses and scores converge as votes arrive.
 */
export function useVectorCompletion({
  prompt,
  responses,
  swarmOwner,
  swarmRepo,
  swarmCommit,
}: UseVectorCompletionParams): VectorCompletionResult {
  const [chunk, setChunk] = useState<Chunk | null>(null);
  const [state, setState] = useState<VectorCompletionState>("idle");
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  const run = useCallback(async () => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;

    setState("streaming");
    setError(null);
    setChunk(null);

    try {
      const client = getClient();
      const stream = await vectorCompletionsCreateVectorCompletion(
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
        { signal: controller.signal },
      );

      let acc: Chunk | null = null;
      for await (const c of stream) {
        if (controller.signal.aborted) break;
        if (acc === null) {
          acc = c;
        } else {
          const [merged] = vectorCompletionsResponseStreamingVectorCompletionChunkMerged(acc, c);
          acc = merged;
        }
        setChunk(acc);
      }

      if (!controller.signal.aborted) {
        setState("done");
      }
    } catch (err: unknown) {
      if ((err as Error)?.name === "AbortError") return;
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      setState("error");
    }
  }, [prompt, responses, swarmOwner, swarmRepo, swarmCommit]);

  const abort = useCallback(() => {
    abortRef.current?.abort();
    setState("idle");
  }, []);

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
    model: v.agent,
    vote: v.vote,
    weight: v.weight,
  }));

  return {
    votes,
    scores: chunk.scores ?? [],
    weights: chunk.weights ?? [],
  };
}
