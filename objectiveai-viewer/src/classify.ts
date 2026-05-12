import {
  AgentCompletionCreateParamsSchema,
  FunctionExecutionCreateParamsSchema,
  FunctionInventionRecursiveCreateParamsSchema,
  LaboratoryExecutionCreateParamsSchema,
  ResponseErrorSchema,
  AgentCompletionsResponseStreamingAgentCompletionChunkSchema,
  FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema,
  FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema,
  LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkSchema,
} from "./types";
import type {
  AgentCompletionEvent,
  FunctionExecutionEvent,
  FunctionInventionRecursiveEvent,
  LaboratoryExecutionEvent,
} from "./types";

export function classifyAgentCompletion(payload: unknown): AgentCompletionEvent | null {
  const beginParse = AgentCompletionCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  const chunkParse = AgentCompletionsResponseStreamingAgentCompletionChunkSchema.safeParse(payload);
  if (chunkParse.success) return { type: "chunk", data: chunkParse.data };
  return null;
}

export function classifyFunctionExecution(payload: unknown): FunctionExecutionEvent | null {
  const beginParse = FunctionExecutionCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  const chunkParse = FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.safeParse(payload);
  if (chunkParse.success) return { type: "chunk", data: chunkParse.data };
  return null;
}

export function classifyFunctionInventionRecursive(payload: unknown): FunctionInventionRecursiveEvent | null {
  const beginParse = FunctionInventionRecursiveCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  const chunkParse = FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema.safeParse(payload);
  if (chunkParse.success) return { type: "chunk", data: chunkParse.data };
  return null;
}

export function classifyLaboratoryExecution(payload: unknown): LaboratoryExecutionEvent | null {
  const beginParse = LaboratoryExecutionCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  const chunkParse = LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkSchema.safeParse(payload);
  if (chunkParse.success) return { type: "chunk", data: chunkParse.data };
  return null;
}
