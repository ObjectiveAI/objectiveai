"use client";

import { useState, useRef, useCallback } from "react";
import {
  functionsInventionsRecursiveCreateFunctionInventionRecursive,
  functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged,
} from "objectiveai";
import type {
  FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk,
  FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsStreaming,
} from "objectiveai";
import { getClient } from "./sdk";

type Chunk = FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk;
type CreateParams = FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsStreaming;

export type InventionState = "idle" | "streaming" | "done" | "error";

export type StepName = "essay" | "input_schema" | "essay_tasks" | "tasks" | "description";

export const STEP_ORDER: StepName[] = ["essay", "input_schema", "essay_tasks", "tasks", "description"];

export interface InventionResult {
  /** Raw accumulated chunk */
  chunk: Chunk | null;
  /** Current step being processed */
  currentStep: StepName | null;
  /** Lifecycle state */
  state: InventionState;
  /** Error message */
  error: string | null;
  /** Start invention */
  invent: () => void;
  /** Abort */
  abort: () => void;
}

interface UseInventionParams {
  /** Full request body (excluding stream) — caller builds the correct shape */
  params: Omit<CreateParams, "stream">;
  /** Auto-start on mount */
  autoStart?: boolean;
}

/**
 * Hook for streaming a recursive function invention via the ObjectiveAI SDK.
 *
 * Accumulates chunks via the SDK's merge system. Exposes the current invention
 * step and raw chunk data for visualization.
 *
 * Note: Requires API access with auth. Will error if API is unavailable.
 */
export function useInvention({
  params,
  autoStart = false,
}: UseInventionParams): InventionResult {
  const [chunk, setChunk] = useState<Chunk | null>(null);
  const [state, setState] = useState<InventionState>("idle");
  const [error, setError] = useState<string | null>(null);
  const [currentStep, setCurrentStep] = useState<StepName | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  const invent = useCallback(async () => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;

    setState("streaming");
    setError(null);
    setChunk(null);
    setCurrentStep("essay");

    try {
      const client = getClient();
      const stream = await functionsInventionsRecursiveCreateFunctionInventionRecursive(
        client,
        {
          ...params,
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
          const [merged] = functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged(acc, c);
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
  }, [params]);

  const abort = useCallback(() => {
    abortRef.current?.abort();
    setState("idle");
  }, []);

  return { chunk, currentStep, state, error, invent, abort };
}
