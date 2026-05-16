import { ObjectiveAI, type RequestOptions } from "../../../client";
import { Stream } from "../../../stream";
import type { FunctionsProfilesComputationsRequestFunctionProfileComputationCreateParams } from "./request/functionProfileComputationCreateParams";
import type { FunctionsProfilesComputationsResponseUnaryFunctionProfileComputation } from "./response/unary/functionProfileComputation";
import type { FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk } from "./response/streaming/functionProfileComputationChunk";

export function functionsProfilesComputationsComputeProfile(
  client: ObjectiveAI,
  body: FunctionsProfilesComputationsRequestFunctionProfileComputationCreateParams & { stream: true },
  options?: RequestOptions,
): Promise<Stream<FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk>>;
export function functionsProfilesComputationsComputeProfile(
  client: ObjectiveAI,
  body: FunctionsProfilesComputationsRequestFunctionProfileComputationCreateParams & { stream?: false | null },
  options?: RequestOptions,
): Promise<FunctionsProfilesComputationsResponseUnaryFunctionProfileComputation>;
export function functionsProfilesComputationsComputeProfile(
  client: ObjectiveAI,
  body: FunctionsProfilesComputationsRequestFunctionProfileComputationCreateParams,
  options?: RequestOptions,
): Promise<
  | Stream<FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk>
  | FunctionsProfilesComputationsResponseUnaryFunctionProfileComputation
> {
  if (body.stream) {
    return client.post_streaming<FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk>(
      "functions/profiles/compute",
      body,
      options,
    );
  }
  return client.post_unary<FunctionsProfilesComputationsResponseUnaryFunctionProfileComputation>(
    "functions/profiles/compute",
    body,
    options,
  );
}
