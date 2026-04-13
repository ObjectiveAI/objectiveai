"use client";

import { FunctionTree } from "@objectiveai/function-tree";
import type { InputFunctionExecution, InputFunctionDefinition, InputProfile } from "@objectiveai/function-tree/core";
import { ExecutionResult } from "@/components/ExecutionResult";
import { JudgmentStack } from "@/components/JudgmentStack";
import type { FunctionDefinition } from "@/lib/functions/types";
import type { ProfileMeta } from "@/lib/profiles/types";
import { TerminalPanelV2, DecompositionView, VoteMatrix, VoteMatrixV2, SuperpositionView, ContributionWaterfall } from "./prototypes";

// ── Complete execution ──
const MOCK_EXECUTION: InputFunctionExecution = {
  id: "exec-a7f3b2c1e9d4",
  function: "ObjectiveAI/is-code",
  profile: "profile-standard",
  output: [0.72, 0.28],
  reasoning: {
    choices: [{ message: { content: "The swarm converged on response A with high confidence across model families." } }],
  },
  tasks: [
    {
      index: 0, task_index: 0, task_path: [0],
      scores: [0.72, 0.28],
      votes: [
        { model: "llm-001", vote: [0.9, 0.1], weight: 1.2, from_cache: false, from_rng: false },
        { model: "llm-002", vote: [0.7, 0.3], weight: 1.0, from_cache: true, from_rng: false },
        { model: "llm-003", vote: [0.6, 0.4], weight: 0.8, from_cache: false, from_rng: false },
        { model: "llm-004", vote: [0.8, 0.2], weight: 1.1, from_cache: false, from_rng: true },
        { model: "llm-005", vote: [0.5, 0.5], weight: 0.6, from_cache: false, from_rng: false },
      ],
      completions: [],
    },
    {
      index: 1, task_index: 1, task_path: [1],
      scores: [0.45, 0.35, 0.20],
      votes: [
        { model: "llm-001", vote: [0.5, 0.3, 0.2], weight: 1.2, from_cache: false, from_rng: false },
        { model: "llm-002", vote: [0.4, 0.4, 0.2], weight: 1.0, from_cache: false, from_rng: false },
        { model: "llm-003", vote: [0.3, 0.3, 0.4], weight: 0.8, from_cache: true, from_rng: false },
      ],
      completions: [],
    },
  ],
};

// ── Mid-execution (streaming) — Task 0 has votes, Task 1 still running ──
const MOCK_STREAMING: InputFunctionExecution = {
  id: "exec-b8c4d3e2f1a5",
  function: "ObjectiveAI/is-code",
  profile: "profile-standard",
  tasks: [
    {
      index: 0, task_index: 0, task_path: [0],
      scores: [0.72, 0.28],
      votes: [
        { model: "llm-001", vote: [0.9, 0.1], weight: 1.2, from_cache: false, from_rng: false },
        { model: "llm-002", vote: [0.7, 0.3], weight: 1.0, from_cache: true, from_rng: false },
        { model: "llm-003", vote: [0.6, 0.4], weight: 0.8, from_cache: false, from_rng: false },
      ],
      completions: [],
    },
    {
      index: 1, task_index: 1, task_path: [1],
      scores: [],
      votes: [],
      completions: [
        { model: "llm-001", choices: [{ delta: { content: "Based on the code structure and readability, I would assess" } }] },
      ],
    },
  ],
};

const MOCK_MODEL_NAMES: Record<string, string> = {
  "llm-001": "openai/gpt-4o",
  "llm-002": "anthropic/claude-3.5-sonnet",
  "llm-003": "google/gemini-2.5-flash",
  "llm-004": "meta/llama-3.3-70b",
  "llm-005": "deepseek/deepseek-v3",
};

const MOCK_RESPONSE_LABELS: Record<string, string[]> = {
  "0": ["Yes, code", "Not code"],
  "1": ["Excellent", "Acceptable", "Poor"],
};

const MOCK_DEFINITION: InputFunctionDefinition = {
  type: "vector.function",
  tasks: [
    {
      type: "vector.completion",
      responses: ["Yes", "No"],
      messages: [
        { role: "system", content: "Determine whether the given input is source code or a code snippet." },
        { role: "user", content: "{{ input }}" },
      ],
    },
    {
      type: "vector.completion",
      responses: ["Excellent", "Acceptable", "Poor"],
      messages: [
        { role: "system", content: "Rate the quality of the code based on readability, correctness, and style." },
        { role: "user", content: "{{ input }}" },
      ],
    },
  ],
};

const MOCK_PROFILE: InputProfile = {
  description: "Standard profile",
  profile: [0.6, 0.4],
  tasks: [
    {
      ensemble: {
        llms: [
          { model: "gpt-4o", output_mode: "log_probs", top_logprobs: 5 },
          { model: "claude-3.5-sonnet", output_mode: "log_probs", top_logprobs: 5 },
          { model: "gemini-2.5-flash", output_mode: "log_probs", top_logprobs: 5 },
          { model: "llama-3.3-70b", output_mode: "log_probs" },
          { model: "deepseek-v3" },
        ],
      },
      profile: [1.2, 1.0, 0.8, 1.1, 0.6],
    },
    {
      ensemble: {
        llms: [
          { model: "gpt-4o", output_mode: "log_probs", top_logprobs: 5 },
          { model: "claude-3.5-sonnet", output_mode: "log_probs", top_logprobs: 5 },
          { model: "gemini-2.5-flash", output_mode: "log_probs", top_logprobs: 5 },
        ],
      },
      profile: [1.2, 1.0, 0.8],
    },
  ],
};

// ── JudgmentStack native types ──
const JS_DEFINITION: FunctionDefinition = {
  type: "alpha.vector.function",
  description: "Determine whether input is source code and rate its quality",
  tasks: [
    {
      type: "vector.completion",
      responses: ["Yes", "No"],
      messages: [
        { role: "system", content: "Determine whether the given input is source code or a code snippet." },
        { role: "user", content: "{{ input }}" },
      ] as unknown as Record<string, unknown>,
      output: { $jmespath: "output.scores" },
    },
    {
      type: "vector.completion",
      responses: ["Excellent", "Acceptable", "Poor"],
      messages: [
        { role: "system", content: "Rate the quality of the code based on readability, correctness, and style." },
        { role: "user", content: "{{ input }}" },
      ] as unknown as Record<string, unknown>,
      skip: { $jmespath: "input.skip_quality" },
      output: { $starlark: "output['scores'][0]" },
    },
  ],
  input_schema: {
    type: "object",
    properties: { code: { type: "string" }, skip_quality: { type: "boolean" } },
    required: ["code"],
  },
};

const JS_PROFILE: ProfileMeta = {
  remote: "ObjectiveAI/profile-standard",
  owner: "ObjectiveAI",
  repository: "profile-standard",
  commit: "abc123",
  name: "profile-standard",
  description: "Standard ensemble with frontier and mid-tier models",
  kind: "auto",
  llms: [
    { model: "openai/gpt-4o", outputMode: "log_probs", topLogprobs: 5, temperature: null, reasoning: null, count: 1, fallbacks: [] },
    { model: "anthropic/claude-3.5-sonnet", outputMode: "log_probs", topLogprobs: 5, temperature: null, reasoning: null, count: 1, fallbacks: [] },
    { model: "google/gemini-2.5-flash", outputMode: "log_probs", topLogprobs: 5, temperature: null, reasoning: null, count: 1, fallbacks: [] },
    { model: "meta/llama-3.3-70b", outputMode: "log_probs", topLogprobs: null, temperature: null, reasoning: null, count: 1, fallbacks: [] },
    { model: "deepseek/deepseek-v3", outputMode: "default", topLogprobs: null, temperature: null, reasoning: null, count: 1, fallbacks: [] },
  ],
  weights: [1.2, 1.0, 0.8, 1.1, 0.6],
  taskConfigs: [],
  taskWeights: [],
  pairedFunction: null,
  totalAgents: 5,
  tiers: {
    frontier: [
      { llm: { model: "openai/gpt-4o", outputMode: "log_probs", topLogprobs: 5, temperature: null, reasoning: null, count: 1, fallbacks: [] }, weight: 1.2 },
      { llm: { model: "anthropic/claude-3.5-sonnet", outputMode: "log_probs", topLogprobs: 5, temperature: null, reasoning: null, count: 1, fallbacks: [] }, weight: 1.0 },
    ],
    mid: [
      { llm: { model: "google/gemini-2.5-flash", outputMode: "log_probs", topLogprobs: 5, temperature: null, reasoning: null, count: 1, fallbacks: [] }, weight: 0.8 },
      { llm: { model: "meta/llama-3.3-70b", outputMode: "log_probs", topLogprobs: null, temperature: null, reasoning: null, count: 1, fallbacks: [] }, weight: 1.1 },
    ],
    budget: [
      { llm: { model: "deepseek/deepseek-v3", outputMode: "default", topLogprobs: null, temperature: null, reasoning: null, count: 1, fallbacks: [] }, weight: 0.6 },
    ],
  },
};

const JS_EXECUTION = {
  id: "exec-a7f3b2c1e9d4",
  function: "ObjectiveAI/is-code",
  profile: "profile-standard",
  output: [0.72, 0.28] as number[],
  reasoning: { choices: [{ message: { content: "The swarm converged on response A with high confidence across model families." } }] },
  tasks: [
    {
      task_path: [0],
      scores: [0.72, 0.28],
      votes: [
        { model: "llm-001", vote: [0.9, 0.1], weight: 1.2, from_cache: false, from_rng: false },
        { model: "llm-002", vote: [0.7, 0.3], weight: 1.0, from_cache: true, from_rng: false },
        { model: "llm-003", vote: [0.6, 0.4], weight: 0.8, from_cache: false, from_rng: false },
        { model: "llm-004", vote: [0.8, 0.2], weight: 1.1, from_cache: false, from_rng: true },
        { model: "llm-005", vote: [0.5, 0.5], weight: 0.6, from_cache: false, from_rng: false },
      ],
    },
    {
      task_path: [1],
      scores: [0.45, 0.35, 0.20],
      votes: [
        { model: "llm-001", vote: [0.5, 0.3, 0.2], weight: 1.2, from_cache: false, from_rng: false },
        { model: "llm-002", vote: [0.4, 0.4, 0.2], weight: 1.0, from_cache: false, from_rng: false },
        { model: "llm-003", vote: [0.3, 0.3, 0.4], weight: 0.8, from_cache: true, from_rng: false },
      ],
    },
  ],
};

const sectionLabel: React.CSSProperties = {
  fontFamily: '"JetBrains Mono", monospace',
  fontSize: 11,
  color: "#78716c",
  marginBottom: 8,
  letterSpacing: "0.05em",
  textTransform: "uppercase" as const,
};

const protoProps = {
  execution: MOCK_EXECUTION,
  definition: MOCK_DEFINITION,
  profile: MOCK_PROFILE,
  modelNames: MOCK_MODEL_NAMES,
  responseLabels: MOCK_RESPONSE_LABELS,
};

const streamProps = {
  execution: MOCK_STREAMING,
  definition: MOCK_DEFINITION,
  profile: MOCK_PROFILE,
  modelNames: MOCK_MODEL_NAMES,
  responseLabels: MOCK_RESPONSE_LABELS,
};

export default function Demo() {
  return (
    <main style={{ padding: 24, background: "#1B1B1B", minHeight: "100vh" }}>
      <h2 style={{
        fontFamily: "var(--font-mono)",
        fontSize: 13,
        color: "#9B9BAB",
        marginBottom: 32,
      }}>
        execution visualization — prototype comparison
      </h2>

      {/* ----------------------------------------------------------------- */}
      {/* JUDGMENT STACK — structural (definition + profile, no execution)  */}
      {/* ----------------------------------------------------------------- */}
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>judgment stack — structural (definition + profile only)</p>
        <JudgmentStack
          definition={JS_DEFINITION}
          profile={JS_PROFILE}
          modelNames={MOCK_MODEL_NAMES}
        />
      </div>

      {/* ----------------------------------------------------------------- */}
      {/* JUDGMENT STACK — execution (full vote data)                       */}
      {/* ----------------------------------------------------------------- */}
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>judgment stack — execution (with vote data)</p>
        <JudgmentStack
          definition={JS_DEFINITION}
          execution={JS_EXECUTION}
          profile={JS_PROFILE}
          modelNames={MOCK_MODEL_NAMES}
        />
      </div>

      {/* ----------------------------------------------------------------- */}
      {/* LEAD: Vote Matrix V2 — refined, full labels, contribution bars    */}
      {/* ----------------------------------------------------------------- */}
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>execution result — promoted component</p>
        <ExecutionResult {...protoProps} />
      </div>
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>execution result — streaming</p>
        <VoteMatrixV2 {...streamProps} />
      </div>

      {/* ----------------------------------------------------------------- */}
      {/* Vote Matrix V1 for comparison                                     */}
      {/* ----------------------------------------------------------------- */}
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>5. vote matrix v1 (comparison)</p>
        <VoteMatrix {...protoProps} />
      </div>

      {/* ----------------------------------------------------------------- */}
      {/* NEW: Superposition — interference pattern / signal stacking       */}
      {/* ----------------------------------------------------------------- */}
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>6. superposition — waves that sum to judgment</p>
        <SuperpositionView {...protoProps} />
      </div>
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>6. superposition — streaming</p>
        <SuperpositionView {...streamProps} />
      </div>

      {/* ----------------------------------------------------------------- */}
      {/* NEW: Contribution Waterfall — who built this judgment?            */}
      {/* ----------------------------------------------------------------- */}
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>7. contribution waterfall — who built this judgment</p>
        <ContributionWaterfall {...protoProps} />
      </div>
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>7. contribution waterfall — streaming</p>
        <ContributionWaterfall {...streamProps} />
      </div>

      {/* ----------------------------------------------------------------- */}
      {/* Previous prototypes for comparison                                */}
      {/* ----------------------------------------------------------------- */}
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>1b. terminal panel v2 (previous)</p>
        <TerminalPanelV2 {...protoProps} />
      </div>

      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>4. decomposition (previous)</p>
        <DecompositionView {...protoProps} />
      </div>

      {/* Canvas tree for reference */}
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>canvas tree (reference)</p>
        <FunctionTree
          data={MOCK_EXECUTION}
          definition={MOCK_DEFINITION}
          profile={MOCK_PROFILE}
          modelNames={MOCK_MODEL_NAMES}
          responseLabels={MOCK_RESPONSE_LABELS}
          height={500}
          borderless
          config={{ theme: "dark" }}
        />
      </div>
    </main>
  );
}
