import z from "zod";
import { ObjectiveAI, type RequestOptions } from "../../client";
import { Stream } from "../../stream";
import {
  AgentCompletionsRequestAgentCompletionCreateParamsSchema,
  type AgentCompletionsRequestAgentCompletionCreateParams,
} from "./request/agentCompletionCreateParams";
import type { AgentCompletionsRequestAgentCompletionNotifyParams } from "./request/agentCompletionNotifyParams";
import type { AgentCompletionsResponseUnaryAgentCompletion } from "./response/unary/agentCompletion";
import type { AgentCompletionsResponseStreamingAgentCompletionChunk } from "./response/streaming/agentCompletionChunk";

export const AgentCompletionsRequestAgentCompletionCreateParamsStreamingSchema =
  AgentCompletionsRequestAgentCompletionCreateParamsSchema.extend({
    stream: z.literal(true),
  });
export type AgentCompletionsRequestAgentCompletionCreateParamsStreaming = z.infer<
  typeof AgentCompletionsRequestAgentCompletionCreateParamsStreamingSchema
>;

export const AgentCompletionsRequestAgentCompletionCreateParamsUnarySchema =
  AgentCompletionsRequestAgentCompletionCreateParamsSchema.extend({
    stream: z.literal(false).optional().nullable(),
  });
export type AgentCompletionsRequestAgentCompletionCreateParamsUnary = z.infer<
  typeof AgentCompletionsRequestAgentCompletionCreateParamsUnarySchema
>;

export function agentCompletionsCreateAgentCompletion(
  client: ObjectiveAI,
  body: AgentCompletionsRequestAgentCompletionCreateParamsStreaming,
  options?: RequestOptions,
): Promise<Stream<AgentCompletionsResponseStreamingAgentCompletionChunk>>;
export function agentCompletionsCreateAgentCompletion(
  client: ObjectiveAI,
  body: AgentCompletionsRequestAgentCompletionCreateParamsUnary,
  options?: RequestOptions,
): Promise<AgentCompletionsResponseUnaryAgentCompletion>;
export function agentCompletionsCreateAgentCompletion(
  client: ObjectiveAI,
  body: AgentCompletionsRequestAgentCompletionCreateParams,
  options?: RequestOptions,
): Promise<
  | Stream<AgentCompletionsResponseStreamingAgentCompletionChunk>
  | AgentCompletionsResponseUnaryAgentCompletion
> {
  if (body.stream) {
    return client.post_streaming<AgentCompletionsResponseStreamingAgentCompletionChunk>(
      "/agent/completions",
      body,
      options,
    );
  }
  return client.post_unary<AgentCompletionsResponseUnaryAgentCompletion>(
    "/agent/completions",
    body,
    options,
  );
}

/**
 * Notify a running agent completion with a user message.
 *
 * Pushes a `RichContent` payload at the agent completion identified
 * by `params.response_id`; the api queues it and surfaces it to the
 * model on its next natural inspection point. There is no response
 * body — any 2xx status is the success signal.
 */
export function agentCompletionsNotifyAgentCompletion(
  client: ObjectiveAI,
  body: AgentCompletionsRequestAgentCompletionNotifyParams,
  options?: RequestOptions,
): Promise<void> {
  return client.post_unary_no_response(
    "/agent/completions/notify",
    body,
    options,
  );
}
