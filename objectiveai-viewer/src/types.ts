import { z } from "zod";
import {
  AgentCompletionsRequestAgentCompletionCreateParamsSchema,
  AgentCompletionsResponseStreamingAgentCompletionChunkSchema,
  FunctionsExecutionsRequestFunctionExecutionCreateParamsSchema,
  FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema,
  FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsSchema,
  FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema,
  LaboratoriesExecutionsRequestLaboratoryExecutionCreateParamsSchema,
  LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkSchema,
  ErrorResponseErrorSchema,
} from "objectiveai";
import type {
  AgentCompletionsResponseStreamingAgentCompletionChunk,
  FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk,
  LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk,
} from "objectiveai";

export const AgentCompletionCreateParamsSchema = AgentCompletionsRequestAgentCompletionCreateParamsSchema.extend({
  id: z.string(),
});
export type AgentCompletionCreateParams = z.infer<typeof AgentCompletionCreateParamsSchema>;

export const FunctionExecutionCreateParamsSchema = FunctionsExecutionsRequestFunctionExecutionCreateParamsSchema.extend({
  id: z.string(),
});
export type FunctionExecutionCreateParams = z.infer<typeof FunctionExecutionCreateParamsSchema>;

export const FunctionInventionRecursiveCreateParamsSchema = FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsSchema.extend({
  id: z.string(),
});
export type FunctionInventionRecursiveCreateParams = z.infer<typeof FunctionInventionRecursiveCreateParamsSchema>;

export const LaboratoryExecutionCreateParamsSchema = LaboratoriesExecutionsRequestLaboratoryExecutionCreateParamsSchema.extend({
  id: z.string(),
});
export type LaboratoryExecutionCreateParams = z.infer<typeof LaboratoryExecutionCreateParamsSchema>;

export const ResponseErrorSchema = ErrorResponseErrorSchema.extend({
  id: z.string(),
});
export type ResponseError = z.infer<typeof ResponseErrorSchema>;

export { AgentCompletionsResponseStreamingAgentCompletionChunkSchema };
export { FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema };
export { FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema };
export { LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkSchema };

export type AgentCompletionEvent =
  | { type: "begin"; data: AgentCompletionCreateParams }
  | { type: "chunk"; data: AgentCompletionsResponseStreamingAgentCompletionChunk }
  | { type: "error"; data: ResponseError };

export type FunctionExecutionEvent =
  | { type: "begin"; data: FunctionExecutionCreateParams }
  | { type: "chunk"; data: FunctionsExecutionsResponseStreamingFunctionExecutionChunk }
  | { type: "error"; data: ResponseError };

export type FunctionInventionRecursiveEvent =
  | { type: "begin"; data: FunctionInventionRecursiveCreateParams }
  | { type: "chunk"; data: FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk }
  | { type: "error"; data: ResponseError };

export type LaboratoryExecutionEvent =
  | { type: "begin"; data: LaboratoryExecutionCreateParams }
  | { type: "chunk"; data: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk }
  | { type: "error"; data: ResponseError };

export interface AgentCompletionEntry {
  kind: "agent-completion";
  id: string;
  request: AgentCompletionCreateParams;
  chunk: AgentCompletionsResponseStreamingAgentCompletionChunk | null;
  error: ResponseError | null;
}

export interface FunctionExecutionEntry {
  kind: "execution";
  id: string;
  request: FunctionExecutionCreateParams;
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk | null;
  error: ResponseError | null;
}

export interface FunctionInventionRecursiveEntry {
  kind: "invention";
  id: string;
  request: FunctionInventionRecursiveCreateParams;
  chunk: FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk | null;
  error: ResponseError | null;
}

export interface LaboratoryExecutionEntry {
  kind: "laboratory";
  id: string;
  request: LaboratoryExecutionCreateParams;
  chunk: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk | null;
  error: ResponseError | null;
}

export type Entry = AgentCompletionEntry | FunctionExecutionEntry | FunctionInventionRecursiveEntry | LaboratoryExecutionEntry;
