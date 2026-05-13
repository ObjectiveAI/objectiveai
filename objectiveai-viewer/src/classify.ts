import {
  AgentCompletionCreateParamsSchema,
  FunctionExecutionCreateParamsSchema,
  FunctionInventionRecursiveCreateParamsSchema,
  LaboratoryExecutionCreateParamsSchema,
  ResponseErrorSchema,
} from "./types";
import type {
  AgentCompletionEvent,
  FunctionExecutionEvent,
  FunctionInventionRecursiveEvent,
  LaboratoryExecutionEvent,
} from "./types";
import type {
  AgentCompletionsResponseStreamingAgentCompletionChunk,
  FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk,
  LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk,
} from "objectiveai";

function hasObject(payload: unknown, ...values: string[]): boolean {
  return typeof payload === "object" && payload !== null &&
    "object" in payload && values.includes((payload as { object: string }).object);
}

export function classifyAgentCompletion(payload: unknown): AgentCompletionEvent | null {
  const beginParse = AgentCompletionCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  if (hasObject(payload, "agent.completion.chunk"))
    return { type: "chunk", data: payload as AgentCompletionsResponseStreamingAgentCompletionChunk };
  return null;
}

export function classifyFunctionExecution(payload: unknown): FunctionExecutionEvent | null {
  const beginParse = FunctionExecutionCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  if (hasObject(payload, "scalar.function.execution.chunk", "vector.function.execution.chunk"))
    return { type: "chunk", data: payload as FunctionsExecutionsResponseStreamingFunctionExecutionChunk };
  return null;
}

export function classifyFunctionInventionRecursive(payload: unknown): FunctionInventionRecursiveEvent | null {
  const beginParse = FunctionInventionRecursiveCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  if (hasObject(payload, "alpha.scalar.function.invention.recursive.chunk", "alpha.vector.function.invention.recursive.chunk"))
    return { type: "chunk", data: payload as FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk };
  return null;
}

export function classifyLaboratoryExecution(payload: unknown): LaboratoryExecutionEvent | null {
  const beginParse = LaboratoryExecutionCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  if (hasObject(payload, "laboratory.execution.chunk"))
    return { type: "chunk", data: payload as LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk };
  return null;
}
