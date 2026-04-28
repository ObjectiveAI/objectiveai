import { apiFetch } from "../client";
import type { SwarmMeta, Agent } from "./types";

interface SwarmListResponse {
  data: Array<{ id: string }>;
}

interface SwarmLlmResponse {
  id: string;
  model: string;
  output_mode: string;
  top_logprobs?: number | null;
  temperature?: number | null;
  count?: number | null;
  fallbacks?: Array<{ id: string; model: string }> | null;
}

interface SwarmRetrieveResponse {
  id: string;
  created: number;
  llms: SwarmLlmResponse[];
}

/** Cache for the swarm list + details */
let swarmCache: { data: SwarmMeta[]; ts: number } | null = null;
const CACHE_TTL = 300_000; // 5 minutes

/** Fetch all swarms with resolved agent details */
export async function fetchAllSwarms(): Promise<SwarmMeta[]> {
  if (swarmCache && Date.now() - swarmCache.ts < CACHE_TTL) {
    return swarmCache.data;
  }

  const list = await apiFetch<SwarmListResponse>("/swarms");

  const results = await Promise.allSettled(
    list.data.map((item) => resolveSwarm(item.id))
  );

  const swarms = results
    .filter(
      (r): r is PromiseFulfilledResult<SwarmMeta> => r.status === "fulfilled"
    )
    .map((r) => r.value);

  swarmCache = { data: swarms, ts: Date.now() };
  return swarms;
}

function parseAgent(llm: SwarmLlmResponse): Agent {
  const fallbacks = llm.fallbacks ?? [];
  return {
    id: llm.id,
    model: llm.model,
    outputMode: llm.output_mode,
    topLogprobs: llm.top_logprobs ?? null,
    temperature: llm.temperature ?? null,
    count: llm.count ?? 1,
    hasFallbacks: fallbacks.length > 0,
    fallbackCount: fallbacks.length,
  };
}

/** Resolve a single swarm by ID */
async function resolveSwarm(id: string): Promise<SwarmMeta> {
  const detail = await apiFetch<SwarmRetrieveResponse>(`/swarms/${id}`);

  const agents = detail.llms.map(parseAgent);
  const totalAgentCount = agents.reduce((sum, a) => sum + a.count, 0);

  return {
    id,
    created: detail.created,
    agents,
    totalAgentCount,
  };
}
