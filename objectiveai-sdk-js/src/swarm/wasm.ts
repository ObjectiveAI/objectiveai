import { validateSwarm } from "../wasm/loader.js";
import type { SwarmSwarmBase } from "./swarmBase";
import type { SwarmSwarm } from "./swarm";
import type { AgentRemoteAgentBaseWithFallbacks } from "../agent/remoteAgentBaseWithFallbacks";

export function wasmSwarmValidateSwarm(
  swarm: SwarmSwarmBase,
  remoteAgents?: Record<string, AgentRemoteAgentBaseWithFallbacks>,
): SwarmSwarm {
  return JSON.parse(validateSwarm(swarm, remoteAgents));
}
