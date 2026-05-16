import z from "zod";
import { ObjectiveAI, type RequestOptions } from "../../client";
import { Stream } from "../../stream";
import {
  VectorCompletionsRequestVectorCompletionCreateParamsSchema,
  type VectorCompletionsRequestVectorCompletionCreateParams,
} from "./request/vectorCompletionCreateParams";
import type { VectorCompletionsResponseUnaryVectorCompletion } from "./response/unary/vectorCompletion";
import type { VectorCompletionsResponseStreamingVectorCompletionChunk } from "./response/streaming/vectorCompletionChunk";

export const VectorCompletionsRequestVectorCompletionCreateParamsStreamingSchema =
  VectorCompletionsRequestVectorCompletionCreateParamsSchema.extend({
    stream: z.literal(true),
  });
export type VectorCompletionsRequestVectorCompletionCreateParamsStreaming = z.infer<
  typeof VectorCompletionsRequestVectorCompletionCreateParamsStreamingSchema
>;

export const VectorCompletionsRequestVectorCompletionCreateParamsUnarySchema =
  VectorCompletionsRequestVectorCompletionCreateParamsSchema.extend({
    stream: z.literal(false).optional().nullable(),
  });
export type VectorCompletionsRequestVectorCompletionCreateParamsUnary = z.infer<
  typeof VectorCompletionsRequestVectorCompletionCreateParamsUnarySchema
>;

export function vectorCompletionsCreateVectorCompletion(
  client: ObjectiveAI,
  body: VectorCompletionsRequestVectorCompletionCreateParamsStreaming,
  options?: RequestOptions,
): Promise<Stream<VectorCompletionsResponseStreamingVectorCompletionChunk>>;
export function vectorCompletionsCreateVectorCompletion(
  client: ObjectiveAI,
  body: VectorCompletionsRequestVectorCompletionCreateParamsUnary,
  options?: RequestOptions,
): Promise<VectorCompletionsResponseUnaryVectorCompletion>;
export function vectorCompletionsCreateVectorCompletion(
  client: ObjectiveAI,
  body: VectorCompletionsRequestVectorCompletionCreateParams,
  options?: RequestOptions,
): Promise<
  | Stream<VectorCompletionsResponseStreamingVectorCompletionChunk>
  | VectorCompletionsResponseUnaryVectorCompletion
> {
  if (body.stream) {
    return client.post_streaming<VectorCompletionsResponseStreamingVectorCompletionChunk>(
      "/vector/completions",
      body,
      options,
    );
  }
  return client.post_unary<VectorCompletionsResponseUnaryVectorCompletion>(
    "/vector/completions",
    body,
    options,
  );
}
