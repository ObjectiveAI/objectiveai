import z from "zod";
import { ObjectiveAI, type RequestOptions } from "../../client";
import { Stream } from "../../stream";
import {
  FunctionsInventionsRequestFunctionInventionCreateParamsSchema,
  type FunctionsInventionsRequestFunctionInventionCreateParams,
} from "./request/functionInventionCreateParams";
import type { FunctionsInventionsResponseUnaryFunctionInvention } from "./response/unary/functionInvention";
import type { FunctionsInventionsResponseStreamingFunctionInventionChunk } from "./response/streaming/functionInventionChunk";

export const FunctionsInventionsRequestFunctionInventionCreateParamsStreamingSchema =
  FunctionsInventionsRequestFunctionInventionCreateParamsSchema.extend({
    stream: z.literal(true),
  });
export type FunctionsInventionsRequestFunctionInventionCreateParamsStreaming = z.infer<
  typeof FunctionsInventionsRequestFunctionInventionCreateParamsStreamingSchema
>;

export const FunctionsInventionsRequestFunctionInventionCreateParamsUnarySchema =
  FunctionsInventionsRequestFunctionInventionCreateParamsSchema.extend({
    stream: z.literal(false).optional().nullable(),
  });
export type FunctionsInventionsRequestFunctionInventionCreateParamsUnary = z.infer<
  typeof FunctionsInventionsRequestFunctionInventionCreateParamsUnarySchema
>;

export function functionsInventionsCreateFunctionInvention(
  client: ObjectiveAI,
  body: FunctionsInventionsRequestFunctionInventionCreateParamsStreaming,
  options?: RequestOptions,
): Promise<Stream<FunctionsInventionsResponseStreamingFunctionInventionChunk>>;
export function functionsInventionsCreateFunctionInvention(
  client: ObjectiveAI,
  body: FunctionsInventionsRequestFunctionInventionCreateParamsUnary,
  options?: RequestOptions,
): Promise<FunctionsInventionsResponseUnaryFunctionInvention>;
export function functionsInventionsCreateFunctionInvention(
  client: ObjectiveAI,
  body: FunctionsInventionsRequestFunctionInventionCreateParams,
  options?: RequestOptions,
): Promise<
  | Stream<FunctionsInventionsResponseStreamingFunctionInventionChunk>
  | FunctionsInventionsResponseUnaryFunctionInvention
> {
  if (body.stream) {
    return client.post_streaming<FunctionsInventionsResponseStreamingFunctionInventionChunk>(
      "/functions/inventions",
      body,
      options,
    );
  }
  return client.post_unary<FunctionsInventionsResponseUnaryFunctionInvention>(
    "/functions/inventions",
    body,
    options,
  );
}
