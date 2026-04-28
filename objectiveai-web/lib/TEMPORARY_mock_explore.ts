// TEMPORARY — mock data for explore page density testing. Delete this file when done.

import type { FunctionMeta } from "@/lib/functions/types";
import type { ProfileMeta, ProfileLlm, TierBreakdown } from "@/lib/profiles/types";
import type { SwarmMeta } from "@/lib/swarms/types";

function llm(model: string, count: number, opts: Partial<ProfileLlm> = {}): ProfileLlm {
  return {
    model,
    outputMode: opts.outputMode ?? "text",
    topLogprobs: opts.topLogprobs ?? null,
    temperature: opts.temperature ?? null,
    reasoning: opts.reasoning ?? null,
    count,
    fallbacks: [],
  };
}

function tiers(
  frontier: { llm: ProfileLlm; weight: number }[],
  mid: { llm: ProfileLlm; weight: number }[],
  budget: { llm: ProfileLlm; weight: number }[],
): TierBreakdown {
  return { frontier, mid, budget };
}

export const MOCK_FUNCTIONS: FunctionMeta[] = [
  {
    remote: "github.com",
    owner: "objective-ai",
    repository: "content-safety",
    commit: "a1b2c3d",
    name: "content-safety",
    type: "scalar.branch",
    category: "scalar",
    depth: "branch",
    description: "Multi-signal content safety classifier across toxicity, bias, and factual grounding",
    taskCount: 4,
    subFunctions: ["toxicity-check", "bias-scan", "factual-grounding"],
  },
  {
    remote: "github.com",
    owner: "objective-ai",
    repository: "essay-quality",
    commit: "d4e5f6a",
    name: "essay-quality",
    type: "scalar.leaf",
    category: "scalar",
    depth: "leaf",
    description: "Holistic essay quality scoring across structure, argumentation, and clarity",
    taskCount: 1,
    subFunctions: [],
  },
  {
    remote: "github.com",
    owner: "objective-ai",
    repository: "translation-rank",
    commit: "b7c8d9e",
    name: "translation-rank",
    type: "vector.leaf",
    category: "vector",
    depth: "leaf",
    description: "Ranks candidate translations by fluency, accuracy, and cultural fit",
    taskCount: 1,
    subFunctions: [],
  },
  {
    remote: "github.com",
    owner: "objective-ai",
    repository: "hiring-screen",
    commit: "e0f1a2b",
    name: "hiring-screen",
    type: "scalar.branch",
    category: "scalar",
    depth: "branch",
    description: "Structured candidate evaluation across technical skill, communication, and role fit",
    taskCount: 5,
    subFunctions: ["technical-depth", "communication-clarity", "role-alignment", "culture-signal"],
  },
  {
    remote: "github.com",
    owner: "objective-ai",
    repository: "image-preference",
    commit: "c3d4e5f",
    name: "image-preference",
    type: "vector.leaf",
    category: "vector",
    depth: "leaf",
    description: "Visual preference ranking for generated images given a target prompt",
    taskCount: 1,
    subFunctions: [],
  },
  {
    remote: "github.com",
    owner: "objective-ai",
    repository: "code-review",
    commit: "f6a7b8c",
    name: "code-review",
    type: "scalar.branch",
    category: "scalar",
    depth: "branch",
    description: "Automated code review scoring correctness, readability, and security posture",
    taskCount: 3,
    subFunctions: ["correctness", "readability"],
  },
  {
    remote: "github.com",
    owner: "objective-ai",
    repository: "sentiment-v2",
    commit: "a9b0c1d",
    name: "sentiment-v2",
    type: "scalar.leaf",
    category: "scalar",
    depth: "leaf",
    description: "Fine-grained sentiment scoring calibrated against human annotation baselines",
    taskCount: 1,
    subFunctions: [],
  },
  {
    remote: "github.com",
    owner: "objective-ai",
    repository: "response-selector",
    commit: "d2e3f4a",
    name: "response-selector",
    type: "vector.branch",
    category: "vector",
    depth: "branch",
    description: "Selects the best response from a candidate pool using multi-axis judgment",
    taskCount: 6,
    subFunctions: ["relevance-score", "helpfulness-score", "safety-gate"],
  },
  {
    remote: "github.com",
    owner: "objective-ai",
    repository: "toxicity-check",
    commit: "b5c6d7e",
    name: "toxicity-check",
    type: "scalar.leaf",
    category: "scalar",
    depth: "leaf",
    description: "Single-axis toxicity classifier with calibrated thresholds",
    taskCount: 1,
    subFunctions: [],
  },
  {
    remote: "github.com",
    owner: "objective-ai",
    repository: "summarization-eval",
    commit: "e8f9a0b",
    name: "summarization-eval",
    type: "scalar.leaf",
    category: "scalar",
    depth: "leaf",
    description: "Evaluates summary faithfulness, coverage, and conciseness against source",
    taskCount: 1,
    subFunctions: [],
  },
  {
    remote: "github.com",
    owner: "objective-ai",
    repository: "ad-creative-rank",
    commit: "c1d2e3f",
    name: "ad-creative-rank",
    type: "vector.leaf",
    category: "vector",
    depth: "leaf",
    description: "Ranks ad creatives by predicted engagement, brand alignment, and clarity",
    taskCount: 1,
    subFunctions: [],
  },
  {
    remote: "github.com",
    owner: "objective-ai",
    repository: "medical-triage",
    commit: "f4a5b6c",
    name: "medical-triage",
    type: "scalar.branch",
    category: "scalar",
    depth: "branch",
    description: "Multi-factor triage scoring for clinical intake prioritization",
    taskCount: 4,
    subFunctions: ["urgency-signal", "symptom-match", "history-weight"],
  },
];

const _gpt4o = llm("openai/gpt-4o", 3);
const _gpt4oMini = llm("openai/gpt-4o-mini", 5);
const _claude35 = llm("anthropic/claude-3.5-sonnet", 2);
const _claude3h = llm("anthropic/claude-3-haiku", 4);
const _gemini25f = llm("google/gemini-2.5-flash", 3);
const _gemini25p = llm("google/gemini-2.5-pro", 1);
const _deepseek = llm("deepseek/deepseek-chat", 6);
const _llama70 = llm("meta/llama-3.1-70b", 4);
const _llama8 = llm("meta/llama-3.1-8b", 8);
const _mistral = llm("mistralai/mistral-large", 2);
const _qwen = llm("alibaba/qwen-2.5-72b", 3);
const _command = llm("cohere/command-r-plus", 2);

export const MOCK_PROFILES: ProfileMeta[] = [
  {
    remote: "github.com", owner: "objective-ai", repository: "profile-nano", commit: "aaa",
    name: "nano", description: "Minimal cost. Single budget model, no redundancy.",
    kind: "auto",
    llms: [_llama8],
    weights: [1.0],
    taskConfigs: [], taskWeights: [],
    pairedFunction: null,
    totalAgents: 8,
    tiers: tiers([], [], [{ llm: _llama8, weight: 1.0 }]),
  },
  {
    remote: "github.com", owner: "objective-ai", repository: "profile-mini", commit: "bbb",
    name: "mini", description: "Low latency blend. Two budget models for diversity.",
    kind: "auto",
    llms: [_gpt4oMini, _llama8],
    weights: [0.6, 0.4],
    taskConfigs: [], taskWeights: [],
    pairedFunction: null,
    totalAgents: 13,
    tiers: tiers([], [], [{ llm: _gpt4oMini, weight: 0.6 }, { llm: _llama8, weight: 0.4 }]),
  },
  {
    remote: "github.com", owner: "objective-ai", repository: "profile-standard", commit: "ccc",
    name: "standard", description: "Balanced judgment. Frontier + mid-tier cross-check.",
    kind: "auto",
    llms: [_gpt4o, _claude35, _gemini25f, _gpt4oMini],
    weights: [0.35, 0.30, 0.20, 0.15],
    taskConfigs: [], taskWeights: [],
    pairedFunction: null,
    totalAgents: 13,
    tiers: tiers(
      [{ llm: _gpt4o, weight: 0.35 }, { llm: _claude35, weight: 0.30 }],
      [{ llm: _gemini25f, weight: 0.20 }],
      [{ llm: _gpt4oMini, weight: 0.15 }],
    ),
  },
  {
    remote: "github.com", owner: "objective-ai", repository: "profile-giga", commit: "ddd",
    name: "giga", description: "High-accuracy swarm. Wide model diversity with learned weights.",
    kind: "auto",
    llms: [_gpt4o, _claude35, _gemini25p, _gemini25f, _deepseek, _llama70, _mistral],
    weights: [0.22, 0.20, 0.18, 0.12, 0.12, 0.09, 0.07],
    taskConfigs: [], taskWeights: [],
    pairedFunction: null,
    totalAgents: 25,
    tiers: tiers(
      [{ llm: _gpt4o, weight: 0.22 }, { llm: _claude35, weight: 0.20 }, { llm: _gemini25p, weight: 0.18 }],
      [{ llm: _gemini25f, weight: 0.12 }, { llm: _deepseek, weight: 0.12 }],
      [{ llm: _llama70, weight: 0.09 }, { llm: _mistral, weight: 0.07 }],
    ),
  },
  {
    remote: "github.com", owner: "objective-ai", repository: "profile-giga-max", commit: "eee",
    name: "giga-max", description: "Maximum coverage. Every available frontier and mid model, deep redundancy.",
    kind: "auto",
    llms: [_gpt4o, _claude35, _gemini25p, _gemini25f, _deepseek, _llama70, _llama8, _mistral, _qwen, _command, _gpt4oMini, _claude3h],
    weights: [0.14, 0.13, 0.12, 0.10, 0.09, 0.08, 0.07, 0.07, 0.06, 0.05, 0.05, 0.04],
    taskConfigs: [], taskWeights: [],
    pairedFunction: null,
    totalAgents: 43,
    tiers: tiers(
      [{ llm: _gpt4o, weight: 0.14 }, { llm: _claude35, weight: 0.13 }, { llm: _gemini25p, weight: 0.12 }],
      [{ llm: _gemini25f, weight: 0.10 }, { llm: _deepseek, weight: 0.09 }, { llm: _llama70, weight: 0.08 }],
      [{ llm: _llama8, weight: 0.07 }, { llm: _mistral, weight: 0.07 }, { llm: _qwen, weight: 0.06 }, { llm: _command, weight: 0.05 }, { llm: _gpt4oMini, weight: 0.05 }, { llm: _claude3h, weight: 0.04 }],
    ),
  },
  {
    remote: "github.com", owner: "objective-ai", repository: "profile-reasoning", commit: "fff",
    name: "reasoning", description: "Reasoning-focused. Weighted toward models with chain-of-thought.",
    kind: "auto",
    llms: [
      llm("openai/o1", 2, { reasoning: true }),
      llm("anthropic/claude-3.5-sonnet", 2, { reasoning: true }),
      llm("google/gemini-2.5-pro", 1, { reasoning: true }),
    ],
    weights: [0.45, 0.35, 0.20],
    taskConfigs: [], taskWeights: [],
    pairedFunction: null,
    totalAgents: 5,
    tiers: tiers(
      [
        { llm: llm("openai/o1", 2, { reasoning: true }), weight: 0.45 },
        { llm: llm("anthropic/claude-3.5-sonnet", 2, { reasoning: true }), weight: 0.35 },
      ],
      [{ llm: llm("google/gemini-2.5-pro", 1, { reasoning: true }), weight: 0.20 }],
      [],
    ),
  },
  {
    remote: "github.com", owner: "objective-ai", repository: "profile-speed", commit: "ggg",
    name: "speed", description: "Latency-optimized. Fast models only, no reasoning overhead.",
    kind: "auto",
    llms: [_gpt4oMini, _claude3h, _gemini25f, _llama8],
    weights: [0.30, 0.28, 0.22, 0.20],
    taskConfigs: [], taskWeights: [],
    pairedFunction: null,
    totalAgents: 20,
    tiers: tiers(
      [],
      [{ llm: _gpt4oMini, weight: 0.30 }, { llm: _claude3h, weight: 0.28 }, { llm: _gemini25f, weight: 0.22 }],
      [{ llm: _llama8, weight: 0.20 }],
    ),
  },
];

let _swarmId = 0;
function swarmId(): string {
  _swarmId++;
  return `swm_${String(_swarmId).padStart(4, "0")}_mock${Math.random().toString(36).slice(2, 8)}`;
}

function agent(model: string, count: number, opts: Partial<{ outputMode: string; topLogprobs: number; temperature: number; fallbackCount: number }> = {}) {
  const hasFb = (opts.fallbackCount ?? 0) > 0;
  return {
    id: `agt_${Math.random().toString(36).slice(2, 10)}`,
    model,
    outputMode: opts.outputMode ?? "text",
    topLogprobs: opts.topLogprobs ?? null,
    temperature: opts.temperature ?? null,
    count,
    hasFallbacks: hasFb,
    fallbackCount: opts.fallbackCount ?? 0,
  };
}

export const MOCK_SWARMS: SwarmMeta[] = [
  {
    id: swarmId(), created: Date.now() - 86400000 * 2,
    agents: [
      agent("openai/gpt-4o", 3, { topLogprobs: 20, temperature: 0.0 }),
      agent("anthropic/claude-3.5-sonnet", 2, { topLogprobs: 5 }),
      agent("google/gemini-2.5-pro", 1, { topLogprobs: 10, fallbackCount: 2 }),
    ],
    totalAgentCount: 6,
  },
  {
    id: swarmId(), created: Date.now() - 86400000 * 5,
    agents: [
      agent("openai/gpt-4o-mini", 5, { topLogprobs: 20, temperature: 0.0 }),
      agent("meta/llama-3.1-8b", 8, { topLogprobs: 20 }),
    ],
    totalAgentCount: 13,
  },
  {
    id: swarmId(), created: Date.now() - 86400000 * 1,
    agents: [
      agent("openai/gpt-4o", 2, { topLogprobs: 20 }),
      agent("anthropic/claude-3.5-sonnet", 2, { topLogprobs: 5, fallbackCount: 1 }),
      agent("google/gemini-2.5-flash", 3, { topLogprobs: 20 }),
      agent("deepseek/deepseek-chat", 4, { topLogprobs: 20, temperature: 0.2 }),
      agent("meta/llama-3.1-70b", 2, { topLogprobs: 20, fallbackCount: 1 }),
    ],
    totalAgentCount: 13,
  },
  {
    id: swarmId(), created: Date.now() - 3600000 * 6,
    agents: [
      agent("openai/gpt-4o", 5, { topLogprobs: 20, temperature: 0.0 }),
      agent("anthropic/claude-3.5-sonnet", 5, { topLogprobs: 5 }),
      agent("google/gemini-2.5-pro", 3, { topLogprobs: 10 }),
      agent("google/gemini-2.5-flash", 4, { topLogprobs: 20 }),
      agent("deepseek/deepseek-chat", 6, { topLogprobs: 20 }),
      agent("meta/llama-3.1-70b", 4, { topLogprobs: 20, fallbackCount: 2 }),
      agent("mistralai/mistral-large", 2, { topLogprobs: 5 }),
      agent("alibaba/qwen-2.5-72b", 3, { topLogprobs: 20 }),
    ],
    totalAgentCount: 32,
  },
  {
    id: swarmId(), created: Date.now() - 86400000 * 10,
    agents: [
      agent("anthropic/claude-3-haiku", 10, { topLogprobs: 5, temperature: 0.0 }),
    ],
    totalAgentCount: 10,
  },
  {
    id: swarmId(), created: Date.now() - 86400000 * 3,
    agents: [
      agent("openai/gpt-4o", 1, { topLogprobs: 20 }),
      agent("openai/gpt-4o-mini", 3, { topLogprobs: 20 }),
      agent("anthropic/claude-3.5-sonnet", 1, { topLogprobs: 5, fallbackCount: 1 }),
      agent("anthropic/claude-3-haiku", 3, { topLogprobs: 5 }),
    ],
    totalAgentCount: 8,
  },
  {
    id: swarmId(), created: Date.now() - 86400000 * 7,
    agents: [
      agent("google/gemini-2.5-flash", 6, { topLogprobs: 20, temperature: 0.1 }),
      agent("meta/llama-3.1-8b", 6, { topLogprobs: 20, temperature: 0.0 }),
      agent("deepseek/deepseek-chat", 4, { topLogprobs: 20 }),
    ],
    totalAgentCount: 16,
  },
  {
    id: swarmId(), created: Date.now() - 86400000 * 4,
    agents: [
      agent("openai/o1", 2, { outputMode: "reasoning", temperature: 1.0 }),
      agent("anthropic/claude-3.5-sonnet", 2, { topLogprobs: 5 }),
    ],
    totalAgentCount: 4,
  },
];
