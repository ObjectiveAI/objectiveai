"use client";

import { useEffect } from "react";
import {
  functionsExecutionsCreateFunctionExecution,
  functionsExecutionsResponseStreamingFunctionExecutionChunkMerged,
} from "@objectiveai/sdk";
import type {
  FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  FunctionsExpressionInputValue,
} from "@objectiveai/sdk";
import { getClient } from "./sdk";
import { useSDKStream } from "./useSDKStream";
import type { StreamState } from "./useSDKStream";

type Chunk = FunctionsExecutionsResponseStreamingFunctionExecutionChunk;
type InputValue = FunctionsExpressionInputValue;

export type ExecutionState = StreamState;

/** A simplified vote for display purposes */
export interface DisplayVote {
  agent: string;
  vote: number[];
  weight: number;
}

/** JudgmentStack-compatible execution shape */
interface JudgmentExecution {
  id?: string;
  function?: string;
  profile?: string;
  output?: number | number[];
  reasoning?: { choices?: Array<{ message?: { content?: string } }> };
  tasks?: Array<{
    task_path?: number[];
    votes?: Array<{ model: string; vote: number[]; weight: number; from_cache?: boolean; from_rng?: boolean }>;
    completions?: Array<Record<string, unknown>>;
    scores?: number[];
    error?: { message?: string } | null;
    output?: number | number[];
  }>;
}

export interface ExecutionResult {
  chunk: Chunk | null;
  votes: DisplayVote[];
  scores: number[];
  weights: number[];
  judgmentExecution: JudgmentExecution | null;
  state: ExecutionState;
  error: string | null;
  execute: () => void;
  abort: () => void;
}

interface UseExecutionParams {
  functionOwner: string;
  functionRepo: string;
  functionCommit?: string | null;
  profileOwner: string;
  profileRepo: string;
  profileCommit?: string | null;
  input: InputValue;
  autoStart?: boolean;
}

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
  const { chunk, state, error, start: execute, abort } = useSDKStream<Chunk>({
    createStream: (signal) => {
      const client = getClient();
      return functionsExecutionsCreateFunctionExecution(
        client,
        {
          function: {
            remote: "github" as const,
            owner: functionOwner,
            repository: functionRepo,
            commit: functionCommit ?? undefined,
          },
          profile: {
            remote: "github" as const,
            owner: profileOwner,
            repository: profileRepo,
            commit: profileCommit ?? undefined,
          },
          input,
          stream: true as const,
        },
        { signal },
      );
    },
    merge: (acc, next) => functionsExecutionsResponseStreamingFunctionExecutionChunkMerged(acc, next)[0],
    deps: [functionOwner, functionRepo, functionCommit, profileOwner, profileRepo, profileCommit, input],
  });

  useEffect(() => {
    if (autoStart) execute();
  }, [autoStart, execute]);

  const { votes, scores, weights } = extractDisplayData(chunk);
  const judgmentExecution = toJudgmentExecution(chunk);

  return { chunk, votes, scores, weights, judgmentExecution, state, error, execute, abort };
}

function extractDisplayData(chunk: Chunk | null): {
  votes: DisplayVote[];
  scores: number[];
  weights: number[];
} {
  if (!chunk) return { votes: [], scores: [], weights: [] };

  for (const task of chunk.tasks) {
    if (!("votes" in task) || !("scores" in task) || !("weights" in task)) continue;
    const vcTask = task as { votes: Array<{ agent_full_id: string; agent_id: string; vote: number[]; weight: number }>; scores: number[]; weights: number[] };
    if (!Array.isArray(vcTask.votes) || !Array.isArray(vcTask.scores) || !Array.isArray(vcTask.weights)) continue;
    const votes: DisplayVote[] = vcTask.votes.map((v) => ({
      agent: v.agent_full_id,
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

function toJudgmentExecution(chunk: Chunk | null): JudgmentExecution | null {
  if (!chunk) return null;

  return {
    tasks: chunk.tasks.map((task, i) => {
      if ("votes" in task && "scores" in task) {
        const vcTask = task as {
          votes: Array<{ agent_full_id: string; agent_id: string; vote: number[]; weight: number; from_cache?: boolean; from_rng?: boolean }>;
          scores: number[];
          completions?: Array<Record<string, unknown>>;
        };
        if (!Array.isArray(vcTask.votes) || !Array.isArray(vcTask.scores)) return { task_path: [i] };
        return {
          task_path: [i],
          votes: vcTask.votes.map((v) => ({
            model: v.agent_full_id,
            vote: v.vote,
            weight: v.weight,
            from_cache: v.from_cache,
            from_rng: v.from_rng,
          })),
          scores: vcTask.scores,
          completions: vcTask.completions,
        };
      }
      return { task_path: [i] };
    }),
  };
}
