import z from "zod";
import { ObjectiveAI, type RequestOptions } from "../../../client";
import { Stream } from "../../../stream";
import {
  FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsSchema,
  type FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParams,
} from "./request/functionInventionRecursiveCreateParams";
import type { FunctionsInventionsRecursiveResponseUnaryFunctionInventionRecursive } from "./response/unary/functionInventionRecursive";
import type { FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk } from "./response/streaming/functionInventionRecursiveChunk";

export const FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsStreamingSchema =
  FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsSchema.extend({
    stream: z.literal(true),
  });
export type FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsStreaming = z.infer<
  typeof FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsStreamingSchema
>;

export const FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsUnarySchema =
  FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsSchema.extend({
    stream: z.literal(false).optional().nullable(),
  });
export type FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsUnary = z.infer<
  typeof FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsUnarySchema
>;

export function functionsInventionsRecursiveCreateFunctionInventionRecursive(
  client: ObjectiveAI,
  body: FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsStreaming,
  options?: RequestOptions,
): Promise<Stream<FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk>>;
export function functionsInventionsRecursiveCreateFunctionInventionRecursive(
  client: ObjectiveAI,
  body: FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsUnary,
  options?: RequestOptions,
): Promise<FunctionsInventionsRecursiveResponseUnaryFunctionInventionRecursive>;
export function functionsInventionsRecursiveCreateFunctionInventionRecursive(
  client: ObjectiveAI,
  body: FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParams,
  options?: RequestOptions,
): Promise<
  | Stream<FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk>
  | FunctionsInventionsRecursiveResponseUnaryFunctionInventionRecursive
> {
  if (body.stream) {
    return client.post_streaming<FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk>(
      "/functions/inventions/recursive",
      body,
      options,
    );
  }
  return client.post_unary<FunctionsInventionsRecursiveResponseUnaryFunctionInventionRecursive>(
    "/functions/inventions/recursive",
    body,
    options,
  );
}
