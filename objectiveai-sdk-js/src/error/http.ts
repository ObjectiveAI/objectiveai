import z from "zod";
import { ObjectiveAI, type RequestOptions } from "../client";
import { Stream } from "../stream";
import {
  ErrorErrorCreateParamsSchema,
  type ErrorErrorCreateParams,
} from "./errorCreateParams";
import type { ErrorErrorResponse } from "./errorResponse";

export const ErrorErrorCreateParamsStreamingSchema =
  ErrorErrorCreateParamsSchema.extend({
    stream: z.literal(true),
  });
export type ErrorErrorCreateParamsStreaming = z.infer<
  typeof ErrorErrorCreateParamsStreamingSchema
>;

export const ErrorErrorCreateParamsUnarySchema =
  ErrorErrorCreateParamsSchema.extend({
    stream: z.literal(false).optional().nullable(),
  });
export type ErrorErrorCreateParamsUnary = z.infer<
  typeof ErrorErrorCreateParamsUnarySchema
>;

export function errorCreateError(
  client: ObjectiveAI,
  body: ErrorErrorCreateParamsStreaming,
  options?: RequestOptions,
): Promise<Stream<ErrorErrorResponse>>;
export function errorCreateError(
  client: ObjectiveAI,
  body: ErrorErrorCreateParamsUnary,
  options?: RequestOptions,
): Promise<ErrorErrorResponse>;
export function errorCreateError(
  client: ObjectiveAI,
  body: ErrorErrorCreateParams,
  options?: RequestOptions,
): Promise<Stream<ErrorErrorResponse> | ErrorErrorResponse> {
  if (body.stream) {
    return client.post_streaming<ErrorErrorResponse>(
      "/error",
      body,
      options,
    );
  }
  return client.post_unary<ErrorErrorResponse>(
    "/error",
    body,
    options,
  );
}
