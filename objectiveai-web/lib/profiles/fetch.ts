import { apiFetch } from "../client";
import type {
  ProfileListItem,
  ProfileMeta,
  ProfileLlm,
  ProfileFallback,
  ProfileTaskConfig,
  TierBreakdown,
} from "./types";

interface RawLlm {
  model: string;
  output_mode: string;
  top_logprobs?: number | null;
  temperature?: number | null;
  reasoning?: { enabled: boolean } | null;
  count?: number | null;
  fallbacks?: RawLlm[] | null;
}

interface RawAutoProfile {
  remote: string;
  owner: string;
  repository: string;
  commit: string;
  description?: string;
  ensemble: { llms: RawLlm[] };
  profile: number[];
}

interface RawTasksProfile {
  remote: string;
  owner: string;
  repository: string;
  commit: string;
  description?: string;
  tasks: Array<{ ensemble: { llms: RawLlm[] }; profile: number[] }>;
  profile: number[];
}

type RawProfile = RawAutoProfile | RawTasksProfile;

const DEFAULT_PROFILE_SLUGS = [
  "profile-nano",
  "profile-mini",
  "profile-standard",
  "profile-giga",
  "profile-giga-max",
] as const;

let defaultCache: { data: ProfileMeta[]; ts: number } | null = null;
const CACHE_TTL = 300_000; // 5 minutes

/** Fetch a profile directly from GitHub (raw.githubusercontent.com) */
async function fetchProfileFromGitHub(owner: string, repo: string): Promise<RawAutoProfile> {
  const url = `https://raw.githubusercontent.com/${owner}/${repo}/main/profile.json`;
  const res = await fetch(url);
  if (!res.ok) throw new Error(`GitHub ${url}: ${res.status}`);
  const data = await res.json();
  return { ...data, remote: "github", owner, repository: repo, commit: "main" };
}

/** Fetch a raw profile, trying API first then falling back to GitHub */
async function fetchRawProfile(owner: string, repo: string): Promise<RawProfile> {
  try {
    return await apiFetch<RawProfile>(`/functions/profiles/github/${owner}/${repo}`);
  } catch {
    return fetchProfileFromGitHub(owner, repo);
  }
}

/** Fetch the 5 official default profiles */
export async function fetchDefaultProfiles(): Promise<ProfileMeta[]> {
  if (defaultCache && Date.now() - defaultCache.ts < CACHE_TTL) {
    return defaultCache.data;
  }

  const results = await Promise.allSettled(
    DEFAULT_PROFILE_SLUGS.map((slug) =>
      fetchRawProfile("ObjectiveAI", slug)
        .then((raw) => parseAutoProfile(raw as RawAutoProfile, slug))
    )
  );

  const profiles = results
    .filter((r): r is PromiseFulfilledResult<ProfileMeta> => r.status === "fulfilled")
    .map((r) => r.value);

  defaultCache = { data: profiles, ts: Date.now() };
  return profiles;
}

function parseLlm(raw: RawLlm): ProfileLlm {
  return {
    model: raw.model,
    outputMode: raw.output_mode,
    topLogprobs: raw.top_logprobs ?? null,
    temperature: raw.temperature ?? null,
    reasoning: raw.reasoning?.enabled ?? null,
    count: raw.count ?? 1,
    fallbacks: (raw.fallbacks ?? []).map(parseFallback),
  };
}

function parseFallback(raw: RawLlm): ProfileFallback {
  return {
    model: raw.model,
    outputMode: raw.output_mode,
    topLogprobs: raw.top_logprobs ?? null,
    reasoning: raw.reasoning?.enabled ?? null,
  };
}

/** Classify an agent into a tier based on its weight */
export function classifyTier(weight: number, maxWeight: number): "frontier" | "mid" | "budget" {
  if (maxWeight === 0) return "budget";
  const ratio = weight / maxWeight;
  if (ratio >= 0.9) return "frontier";
  if (ratio >= 0.2) return "mid";
  return "budget";
}

/** Build tier breakdown from LLMs and weights */
export function buildTiers(llms: ProfileLlm[], weights: number[]): TierBreakdown {
  const maxWeight = Math.max(...weights, 0);
  const tiers: TierBreakdown = { frontier: [], mid: [], budget: [] };

  for (let i = 0; i < llms.length; i++) {
    const w = weights[i] ?? 0;
    const tier = classifyTier(w, maxWeight);
    tiers[tier].push({ llm: llms[i], weight: w });
  }

  return tiers;
}

/** Fetch a single profile by owner/repository slug */
export async function fetchProfileBySlug(owner: string, repository: string): Promise<ProfileMeta> {
  const raw = await fetchRawProfile(owner, repository);
  return parseAutoProfile(raw as RawAutoProfile, repository.replace("profile-", ""));
}

function parseAutoProfile(raw: RawAutoProfile, slug: string): ProfileMeta {
  const llms = raw.ensemble.llms.map(parseLlm);
  const totalAgents = llms.reduce((sum, l) => sum + l.count, 0);
  const tiers = buildTiers(llms, raw.profile);

  return {
    remote: raw.remote,
    owner: raw.owner,
    repository: raw.repository,
    commit: raw.commit,
    name: slug.replace("profile-", ""),
    description: raw.description ?? "",
    kind: "auto",
    llms,
    weights: raw.profile,
    taskConfigs: [],
    taskWeights: [],
    pairedFunction: null,
    totalAgents,
    tiers,
  };
}
