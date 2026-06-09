/** A swarm as returned by the list endpoint */
export interface SwarmListItem {
  id: string;
}

/** Agent configuration within a swarm */
export interface Agent {
  id: string;
  model: string;
  outputMode: string;
  topLogprobs: number | null;
  temperature: number | null;
  count: number;
  hasFallbacks: boolean;
  fallbackCount: number;
}

/** Resolved swarm with its agents */
export interface SwarmMeta {
  id: string;
  created: number;
  agents: Agent[];
  totalAgentCount: number;
}
