import { ObjectiveAI, type RequestOptions } from "../client";
import type { RemotePathCommitOptional } from "../remotePathCommitOptional";
import type { FunctionsListFunctionsRequest } from "./listFunctionsRequest";
import type { FunctionsListFunctionResponse } from "./listFunctionResponse";
import type { FunctionsGetFunctionResponse } from "./getFunctionResponse";
import type { FunctionsUsageFunctionResponse } from "./usageFunctionResponse";
import type { FunctionsListFunctionProfilePairsRequest } from "./listFunctionProfilePairsRequest";
import type { FunctionsListFunctionProfilePairResponse } from "./listFunctionProfilePairResponse";
import type { FunctionsGetFunctionProfilePairUsageRequest } from "./getFunctionProfilePairUsageRequest";
import type { FunctionsUsageFunctionProfilePairResponse } from "./usageFunctionProfilePairResponse";

export function functionsListFunctions(
  client: ObjectiveAI,
  params: FunctionsListFunctionsRequest,
  options?: RequestOptions,
): Promise<FunctionsListFunctionResponse> {
  return client.post_unary<FunctionsListFunctionResponse>("functions/list", params, options);
}

export function functionsGetFunction(
  client: ObjectiveAI,
  params: RemotePathCommitOptional,
  options?: RequestOptions,
): Promise<FunctionsGetFunctionResponse> {
  return client.post_unary<FunctionsGetFunctionResponse>("functions", params, options);
}

export function functionsGetFunctionUsage(
  client: ObjectiveAI,
  params: RemotePathCommitOptional,
  options?: RequestOptions,
): Promise<FunctionsUsageFunctionResponse> {
  return client.post_unary<FunctionsUsageFunctionResponse>("functions/usage", params, options);
}

export function functionsListFunctionProfilePairs(
  client: ObjectiveAI,
  params: FunctionsListFunctionProfilePairsRequest,
  options?: RequestOptions,
): Promise<FunctionsListFunctionProfilePairResponse> {
  return client.post_unary<FunctionsListFunctionProfilePairResponse>("functions/profiles/pairs/list", params, options);
}

export function functionsGetFunctionProfilePairUsage(
  client: ObjectiveAI,
  params: FunctionsGetFunctionProfilePairUsageRequest,
  options?: RequestOptions,
): Promise<FunctionsUsageFunctionProfilePairResponse> {
  return client.post_unary<FunctionsUsageFunctionProfilePairResponse>("functions/profiles/pairs/usage", params, options);
}
