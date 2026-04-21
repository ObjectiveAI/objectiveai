"use client";

import { useState, useEffect } from "react";
import {
  functionsInventionsRecursiveCreateFunctionInventionRecursive,
  functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged,
} from "objectiveai";
import type {
  FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk,
  FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsStreaming,
} from "objectiveai";
import { getClient } from "./sdk";
import { useSDKStream } from "./useSDKStream";
import type { StreamState } from "./useSDKStream";

type Chunk = FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk;
type CreateParams = FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsStreaming;

export type InventionState = StreamState;

export type StepName = "essay" | "input_schema" | "essay_tasks" | "tasks" | "description";

export const STEP_ORDER: StepName[] = ["essay", "input_schema", "essay_tasks", "tasks", "description"];

export interface InventionResult {
  chunk: Chunk | null;
  currentStep: StepName | null;
  state: InventionState;
  error: string | null;
  invent: () => void;
  abort: () => void;
}

interface UseInventionParams {
  params: Omit<CreateParams, "stream">;
  autoStart?: boolean;
}

export function useInvention({
  params,
  autoStart = false,
}: UseInventionParams): InventionResult {
  const [currentStep, setCurrentStep] = useState<StepName | null>(null);

  const { chunk, state, error, start: invent, abort } = useSDKStream<Chunk>({
    createStream: (signal) => {
      const client = getClient();
      return functionsInventionsRecursiveCreateFunctionInventionRecursive(
        client,
        {
          ...params,
          stream: true as const,
        },
        { signal },
      );
    },
    merge: (acc, next) => functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged(acc, next)[0],
    deps: [params],
    onStreamStart: () => setCurrentStep("essay"),
  });

  useEffect(() => {
    if (autoStart) invent();
  }, [autoStart, invent]);

  return { chunk, currentStep, state, error, invent, abort };
}
