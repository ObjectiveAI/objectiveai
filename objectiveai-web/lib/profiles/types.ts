/** A profile as returned by the list endpoint */
export interface ProfileListItem {
  remote: string;
  owner: string;
  repository: string;
  commit: string;
}

/** An LLM within a profile's swarm */
export interface ProfileLlm {
  model: string;
  outputMode: string;
  topLogprobs: number | null;
  temperature: number | null;
  reasoning: boolean | null;
  count: number;
  fallbacks: ProfileFallback[];
}

export interface ProfileFallback {
  model: string;
  outputMode: string;
  topLogprobs: number | null;
  reasoning: boolean | null;
}

/** A task-level swarm config (for tasks-based profiles) */
export interface ProfileTaskConfig {
  llms: ProfileLlm[];
  weights: number[];
}

/** Agent tier classification */
export type AgentTier = "frontier" | "mid" | "budget";

/** Agents grouped by tier */
export interface TierBreakdown {
  frontier: { llm: ProfileLlm; weight: number }[];
  mid: { llm: ProfileLlm; weight: number }[];
  budget: { llm: ProfileLlm; weight: number }[];
}

/** Resolved profile with detail */
export interface ProfileMeta {
  remote: string;
  owner: string;
  repository: string;
  commit: string;
  name: string;
  description: string;
  kind: "auto" | "tasks";
  /** Auto profiles: single swarm + weights */
  llms: ProfileLlm[];
  weights: number[];
  /** Tasks profiles: per-task configs + task-level weights */
  taskConfigs: ProfileTaskConfig[];
  taskWeights: number[];
  /** Paired function (if any) */
  pairedFunction: ProfileListItem | null;
  /** Total effective agent count (sum of all LLM counts) */
  totalAgents: number;
  /** Agents grouped by tier (auto profiles only) */
  tiers: TierBreakdown;
}
