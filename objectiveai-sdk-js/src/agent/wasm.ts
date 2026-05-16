import { validateAgent } from "../wasm/loader.js";
import type { AgentAgentBase } from "./agentBase";
import type { AgentAgent } from "./agent";

export function wasmAgentValidateAgent(agent: AgentAgentBase): AgentAgent {
  return JSON.parse(validateAgent(agent));
}
