import {
  AgentCompletionCreateParamsSchema,
  FunctionExecutionCreateParamsSchema,
  ResponseErrorSchema,
  AgentCompletionsResponseStreamingAgentCompletionChunkSchema,
  FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema,
} from "./types";
import type {
  AgentCompletionEvent,
  FunctionExecutionEvent,
} from "./types";

let _droppedCount = 0;
const _listeners = new Set<(count: number) => void>();

export function getDroppedEventCount(): number { return _droppedCount; }
export function onDroppedCountChange(fn: (count: number) => void): () => void {
  _listeners.add(fn);
  return () => { _listeners.delete(fn); };
}

function recordDrop(subType: string, payload: unknown, errors: { stage: string; issues: unknown[] }[]) {
  _droppedCount++;
  _listeners.forEach((fn) => fn(_droppedCount));
  console.warn(
    `[objectiveai] dropped ${subType} event — failed all classify stages`,
    { payload, errors },
  );
}

export function classifyAgentCompletion(payload: unknown): AgentCompletionEvent | null {
  const beginParse = AgentCompletionCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  const chunkParse = AgentCompletionsResponseStreamingAgentCompletionChunkSchema.safeParse(payload);
  if (chunkParse.success) return { type: "chunk", data: chunkParse.data };
  recordDrop("agent_completions", payload, [
    { stage: "begin", issues: beginParse.error.issues },
    { stage: "chunk", issues: chunkParse.error.issues },
  ]);
  return null;
}

export function classifyFunctionExecution(payload: unknown): FunctionExecutionEvent | null {
  const beginParse = FunctionExecutionCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  const chunkParse = FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.safeParse(payload);
  if (chunkParse.success) return { type: "chunk", data: chunkParse.data };
  recordDrop("functions_executions", payload, [
    { stage: "begin", issues: beginParse.error.issues },
    { stage: "chunk", issues: chunkParse.error.issues },
  ]);
  return null;
}
