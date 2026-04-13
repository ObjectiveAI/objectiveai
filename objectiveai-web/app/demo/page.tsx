"use client";

import { JudgmentStack } from "@/components/JudgmentStack";
import { InventionStream } from "@/components/InventionStream";
import { VectorPlayground } from "@/components/VectorPlayground";
import { DEMO_INVENTION, DEMO_VECTOR_COMPLETION, DEMO_MODEL_NAMES } from "@/lib/demo-data";
import type { FunctionDefinition } from "@/lib/functions/types";
import type { ProfileMeta } from "@/lib/profiles/types";

// ── JudgmentStack data ──

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

export default function Demo() {
  return (
    <main style={{ padding: 24, background: "#1B1B1B", minHeight: "100vh" }}>
      <h2 style={{
        fontFamily: "var(--font-mono)",
        fontSize: 13,
        color: "#9B9BAB",
        marginBottom: 32,
      }}>
        component reference
      </h2>

      {/* JudgmentStack — structural */}
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>judgment stack — structural (definition + profile only)</p>
        <JudgmentStack
          definition={JS_DEFINITION}
          profile={JS_PROFILE}
          modelNames={DEMO_MODEL_NAMES}
        />
      </div>

      {/* JudgmentStack — execution */}
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>judgment stack — execution (with vote data)</p>
        <JudgmentStack
          definition={JS_DEFINITION}
          execution={JS_EXECUTION}
          profile={JS_PROFILE}
          modelNames={DEMO_MODEL_NAMES}
        />
      </div>

      {/* InventionStream — mid-invention */}
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>invention stream — mid-invention (tasks step streaming)</p>
        <InventionStream
          name={DEMO_INVENTION.name}
          currentStep={DEMO_INVENTION.currentStep}
          steps={DEMO_INVENTION.steps}
          state="streaming"
        />
      </div>

      {/* VectorPlayground — interactive */}
      <div style={{ marginBottom: 48 }}>
        <p style={sectionLabel}>vector playground — interactive vote visualization</p>
        <VectorPlayground
          initialPrompt={DEMO_VECTOR_COMPLETION.prompt}
          initialResponses={DEMO_VECTOR_COMPLETION.responses}
        />
      </div>
    </main>
  );
}
