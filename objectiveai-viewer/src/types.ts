import { z } from "zod";
import {
  AgentCompletionsRequestAgentCompletionCreateParamsSchema,
  AgentCompletionsResponseStreamingAgentCompletionChunkSchema,
  FunctionsExecutionsRequestFunctionExecutionCreateParamsSchema,
  FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema,
  ErrorResponseErrorSchema,
} from "@objectiveai/sdk";
import type {
  AgentCompletionsResponseStreamingAgentCompletionChunk,
  FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
} from "@objectiveai/sdk";

export const AgentCompletionCreateParamsSchema = AgentCompletionsRequestAgentCompletionCreateParamsSchema.extend({
  id: z.string(),
});
export type AgentCompletionCreateParams = z.infer<typeof AgentCompletionCreateParamsSchema>;

export const FunctionExecutionCreateParamsSchema = FunctionsExecutionsRequestFunctionExecutionCreateParamsSchema.extend({
  id: z.string(),
});
export type FunctionExecutionCreateParams = z.infer<typeof FunctionExecutionCreateParamsSchema>;

export const ResponseErrorSchema = ErrorResponseErrorSchema.extend({
  id: z.string(),
});
export type ResponseError = z.infer<typeof ResponseErrorSchema>;

export { AgentCompletionsResponseStreamingAgentCompletionChunkSchema };
export { FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema };

export type AgentCompletionEvent =
  | { type: "begin"; data: AgentCompletionCreateParams }
  | { type: "chunk"; data: AgentCompletionsResponseStreamingAgentCompletionChunk }
  | { type: "error"; data: ResponseError };

export type FunctionExecutionEvent =
  | { type: "begin"; data: FunctionExecutionCreateParams }
  | { type: "chunk"; data: FunctionsExecutionsResponseStreamingFunctionExecutionChunk }
  | { type: "error"; data: ResponseError };

export interface AgentCompletionEntry {
  kind: "agent-completion";
  id: string;
  receivedAt: number;
  request: AgentCompletionCreateParams;
  chunk: AgentCompletionsResponseStreamingAgentCompletionChunk | null;
  error: ResponseError | null;
}

export interface FunctionExecutionEntry {
  kind: "execution";
  id: string;
  receivedAt: number;
  request: FunctionExecutionCreateParams;
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk | null;
  error: ResponseError | null;
}

export type Entry = AgentCompletionEntry | FunctionExecutionEntry;

export interface ViewerInboundEvent {
  type: "inbound";
  destination: string;
  sub_type: string;
  value: unknown;
}

export interface ViewerCliCommandEvent {
  type: "cli_command";
  destination: string;
  value: unknown;
}

export interface ViewerApiCallEvent {
  type: "api_call";
  destination: string;
  sub_type: string;
  value: unknown;
}

export type ViewerEvent = ViewerInboundEvent | ViewerCliCommandEvent | ViewerApiCallEvent;
