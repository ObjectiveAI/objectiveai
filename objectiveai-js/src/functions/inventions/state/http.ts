import { ObjectiveAI, type RequestOptions } from "../../../client";
import type { RemotePathCommitOptional } from "../../../remotePathCommitOptional";
import type { FunctionsInventionsStateGetFunctionInventionStateResponse } from "./getFunctionInventionStateResponse";

export function functionsInventionsStateGetFunctionInventionState(
  client: ObjectiveAI,
  params: RemotePathCommitOptional,
  options?: RequestOptions,
): Promise<FunctionsInventionsStateGetFunctionInventionStateResponse> {
  return client.post_unary<FunctionsInventionsStateGetFunctionInventionStateResponse>("functions/inventions/state", params, options);
}
