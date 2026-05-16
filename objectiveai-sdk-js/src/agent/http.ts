import { ObjectiveAI, type RequestOptions } from "../client";
import type { RemotePathCommitOptional } from "../remotePathCommitOptional";
import type { AgentListAgentsRequest } from "./listAgentsRequest";
import type { AgentListAgentResponse } from "./listAgentResponse";
import type { AgentGetAgentResponse } from "./getAgentResponse";
import type { AgentUsageAgentResponse } from "./usageAgentResponse";

export function agentListAgents(
  client: ObjectiveAI,
  params: AgentListAgentsRequest,
  options?: RequestOptions,
): Promise<AgentListAgentResponse> {
  return client.post_unary<AgentListAgentResponse>("agents/list", params, options);
}

export function agentGetAgent(
  client: ObjectiveAI,
  params: RemotePathCommitOptional,
  options?: RequestOptions,
): Promise<AgentGetAgentResponse> {
  return client.post_unary<AgentGetAgentResponse>("agents", params, options);
}

export function agentGetAgentUsage(
  client: ObjectiveAI,
  params: RemotePathCommitOptional,
  options?: RequestOptions,
): Promise<AgentUsageAgentResponse> {
  return client.post_unary<AgentUsageAgentResponse>("agents/usage", params, options);
}
