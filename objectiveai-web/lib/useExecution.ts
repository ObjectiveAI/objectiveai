"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import {
  functionsExecutionsCreateFunctionExecution,
  functionsExecutionsResponseStreamingFunctionExecutionChunkMerged,
} from "objectiveai";
import type {
  FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  FunctionsExpressionInputValue,
} from "objectiveai";
import { getClient } from "./sdk";

type Chunk = FunctionsExecutionsResponseStreamingFunctionExecutionChunk;
type InputValue = FunctionsExpressionInputValue;

export type ExecutionState = "idle" | "streaming" | "done" | "error";

/** A simplified vote for display purposes */
export interface DisplayVote {
  agent: string;
  vote: number[];
  weight: number;
}

export interface ExecutionResult {
  /** Current accumulated chunk (null before first chunk) */
  chunk: Chunk | null;
  /** Simplified vote list extracted from the latest chunk */
  votes: DisplayVote[];
  /** Current scores vector */
  scores: number[];
  /** Current weights vector */
  weights: number[];
  /** Execution lifecycle state */
  state: ExecutionState;
  /** Error message if state is "error" */
  error: string | null;
  /** Start a new execution */
  execute: () => void;
  /** Abort a running execution */
  abort: () => void;
}

interface UseExecutionParams {
  /** Function owner/repository */
  functionOwner: string;
  functionRepo: string;
  functionCommit?: string | null;
  /** Profile owner/repository */
  profileOwner: string;
  profileRepo: string;
  profileCommit?: string | null;
  /** Function input */
  input: InputValue;
  /** Auto-start on mount */
  autoStart?: boolean;
}

/**
 * Hook for streaming a function execution via the ObjectiveAI SDK.
 *
 * Uses functionsExecutionsCreateFunctionExecution with stream: true,
 * accumulating chunks via the SDK's merge system. Extracts votes, scores, and
 * weights from vector completion tasks for easy display.
 */
export function useExecution({
  functionOwner,
  functionRepo,
  functionCommit,
  profileOwner,
  profileRepo,
  profileCommit,
  input,
  autoStart = false,
}: UseExecutionParams): ExecutionResult {
  const [chunk, setChunk] = useState<Chunk | null>(null);
  const [state, setState] = useState<ExecutionState>("idle");
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  const execute = useCallback(async () => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;

    setState("streaming");
    setError(null);
    setChunk(null);

    try {
      const client = getClient();
      const functionRef = {
        remote: "github" as const,
        owner: functionOwner,
        repository: functionRepo,
        ...(functionCommit ? { commit: functionCommit } : {}),
      };
      const profileRef = {
        remote: "github" as const,
        owner: profileOwner,
        repository: profileRepo,
        ...(profileCommit ? { commit: profileCommit } : {}),
      };

      const stream = await functionsExecutionsCreateFunctionExecution(
        client,
        {
          function: functionRef,
          profile: profileRef,
          input,
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
          const [merged] = functionsExecutionsResponseStreamingFunctionExecutionChunkMerged(acc, c);
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
  }, [functionOwner, functionRepo, functionCommit, profileOwner, profileRepo, profileCommit, input]);

  const abort = useCallback(() => {
    abortRef.current?.abort();
    setState("idle");
  }, []);

  useEffect(() => {
    if (autoStart) execute();
    return () => { abortRef.current?.abort(); };
  }, [autoStart, execute]);

  const { votes, scores, weights } = extractDisplayData(chunk);

  return { chunk, votes, scores, weights, state, error, execute, abort };
}

/** Pull votes/scores/weights from the first vector completion task in the chunk */
function extractDisplayData(chunk: Chunk | null): {
  votes: DisplayVote[];
  scores: number[];
  weights: number[];
} {
  if (!chunk) return { votes: [], scores: [], weights: [] };

  for (const task of chunk.tasks) {
    // VectorCompletionTaskChunk has votes/scores/weights; FunctionExecutionTaskChunk does not
    if (!("votes" in task)) continue;
    const vcTask = task as unknown as { votes: Array<{ agent: string; vote: number[]; weight: number }>; scores: number[]; weights: number[] };
    const votes: DisplayVote[] = vcTask.votes.map((v) => ({
      agent: v.agent,
      vote: v.vote,
      weight: v.weight,
    }));
    return {
      votes,
      scores: vcTask.scores,
      weights: vcTask.weights,
    };
  }

  return { votes: [], scores: [], weights: [] };
}
