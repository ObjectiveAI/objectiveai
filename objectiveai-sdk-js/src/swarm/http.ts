import { ObjectiveAI, type RequestOptions } from "../client";
import type { RemotePathCommitOptional } from "../remotePathCommitOptional";
import type { SwarmListSwarmsRequest } from "./listSwarmsRequest";
import type { SwarmListSwarmResponse } from "./listSwarmResponse";
import type { SwarmGetSwarmResponse } from "./getSwarmResponse";
import type { SwarmUsageSwarmResponse } from "./usageSwarmResponse";

export function swarmListSwarms(
  client: ObjectiveAI,
  params: SwarmListSwarmsRequest,
  options?: RequestOptions,
): Promise<SwarmListSwarmResponse> {
  return client.post_unary<SwarmListSwarmResponse>("swarms/list", params, options);
}

export function swarmGetSwarm(
  client: ObjectiveAI,
  params: RemotePathCommitOptional,
  options?: RequestOptions,
): Promise<SwarmGetSwarmResponse> {
  return client.post_unary<SwarmGetSwarmResponse>("swarms", params, options);
}

export function swarmGetSwarmUsage(
  client: ObjectiveAI,
  params: RemotePathCommitOptional,
  options?: RequestOptions,
): Promise<SwarmUsageSwarmResponse> {
  return client.post_unary<SwarmUsageSwarmResponse>("swarms/usage", params, options);
}
