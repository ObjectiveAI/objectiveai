import { ObjectiveAI, type RequestOptions } from "../../client";
import { Stream } from "../../stream";
import type { FunctionsExecutionsRequestFunctionExecutionCreateParams } from "./request/functionExecutionCreateParams";
import type { FunctionsExecutionsResponseUnaryFunctionExecution } from "./response/unary/functionExecution";
import type { FunctionsExecutionsResponseStreamingFunctionExecutionChunk } from "./response/streaming/functionExecutionChunk";

export function functionsExecutionsCreateFunctionExecution(
  client: ObjectiveAI,
  body: FunctionsExecutionsRequestFunctionExecutionCreateParams & { stream: true },
  options?: RequestOptions,
): Promise<Stream<FunctionsExecutionsResponseStreamingFunctionExecutionChunk>>;
export function functionsExecutionsCreateFunctionExecution(
  client: ObjectiveAI,
  body: FunctionsExecutionsRequestFunctionExecutionCreateParams & { stream?: false | null },
  options?: RequestOptions,
): Promise<FunctionsExecutionsResponseUnaryFunctionExecution>;
export function functionsExecutionsCreateFunctionExecution(
  client: ObjectiveAI,
  body: FunctionsExecutionsRequestFunctionExecutionCreateParams,
  options?: RequestOptions,
): Promise<
  | Stream<FunctionsExecutionsResponseStreamingFunctionExecutionChunk>
  | FunctionsExecutionsResponseUnaryFunctionExecution
> {
  if (body.stream) {
    return client.post_streaming<FunctionsExecutionsResponseStreamingFunctionExecutionChunk>(
      "functions/executions",
      body,
      options,
    );
  }
  return client.post_unary<FunctionsExecutionsResponseUnaryFunctionExecution>(
    "functions/executions",
    body,
    options,
  );
}
