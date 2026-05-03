/**
 * Realistic execution data matching SDK FunctionExecutionChunk shape.
 * Used for demo rendering until real API execution is available.
 * Swap to useExecution() output when API is live — zero structural changes.
 */

import type { FunctionExecution } from "@/components/JudgmentStack";

/** Model name lookup for display */
export const DEMO_MODEL_NAMES: Record<string, string> = {
  "llm-001": "openai/gpt-4o",
  "llm-002": "anthropic/claude-3.5-sonnet",
  "llm-003": "google/gemini-2.5-flash",
  "llm-004": "meta/llama-3.3-70b",
  "llm-005": "deepseek/deepseek-v3",
};

/** Completed execution — all tasks have votes and scores */
export const DEMO_EXECUTION_COMPLETE: FunctionExecution = {
  id: "exec-a7f3b2c1e9d4",
  function: "ObjectiveAI/is-code",
  profile: "ObjectiveAI/profile-standard",
  output: [0.72, 0.28],
  reasoning: {
    choices: [{
      message: {
        content: "The swarm converged on response A with high confidence across model families. Frontier models (gpt-4o, claude-3.5-sonnet) showed strongest agreement at 0.9 and 0.7 respectively. Mid-tier models introduced more variance but overall weight distribution favored the majority signal.",
      },
    }],
  },
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
      completions: [
        { model: "llm-001", text: "The input contains clear syntactic markers of source code: function declarations, variable assignments, and control flow structures. The indentation pattern and use of curly braces are consistent with a C-family language. `A`" },
        { model: "llm-002", text: "This appears to be source code. I can identify import statements, type annotations, and a module export pattern typical of TypeScript. The naming conventions follow camelCase for variables and PascalCase for types. `A`" },
        { model: "llm-003", text: "The input has structural elements of code — semicolons, parentheses grouping, and keyword-like tokens. However, some fragments could be pseudocode or documentation examples. Leaning toward source code but with moderate confidence. `A`" },
        { model: "llm-004", text: "Definite source code. The presence of async/await patterns, error handling blocks, and specific API calls indicates this is production-grade application code, not pseudocode. `A`" },
        { model: "llm-005", text: "This could be either source code or formatted pseudocode. The structure resembles code but lacks some typical markers like complete error handling. Uncertain between the two options. `A`" },
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
      completions: [
        { model: "llm-001", text: "The code demonstrates good structure with clear separation of concerns. Variable names are descriptive and the control flow is straightforward. Some minor style inconsistencies but overall quality is above average. `A`" },
        { model: "llm-002", text: "Acceptable quality. The code works but could benefit from better error handling and more consistent formatting. The logic is sound but readability suffers from dense expressions. `B`" },
        { model: "llm-003", text: "The code quality is mixed. While the core logic is correct, there are issues with error handling, inconsistent naming, and missing edge case coverage. Falls below what I'd consider production-ready. `C`" },
      ],
    },
  ],
};

/** Mid-stream execution — Task 0 done, Task 1 still streaming */
export const DEMO_EXECUTION_STREAMING: FunctionExecution = {
  id: "exec-b8c4d3e2f1a5",
  function: "ObjectiveAI/is-code",
  profile: "ObjectiveAI/profile-standard",
  tasks: [
    {
      task_path: [0],
      scores: [0.72, 0.28],
      votes: [
        { model: "llm-001", vote: [0.9, 0.1], weight: 1.2, from_cache: false, from_rng: false },
        { model: "llm-002", vote: [0.7, 0.3], weight: 1.0, from_cache: true, from_rng: false },
        { model: "llm-003", vote: [0.6, 0.4], weight: 0.8, from_cache: false, from_rng: false },
      ],
      completions: [],
    },
    {
      task_path: [1],
      scores: [],
      votes: [],
      completions: [
        { model: "llm-001", choices: [{ delta: { content: "Based on the code structure and readability, I would assess" } }] },
      ],
    },
  ],
};

/** Invention step names for progress display */
export const INVENTION_STEPS = [
  "essay",
  "input_schema",
  "essay_tasks",
  "tasks",
  "description",
] as const;

export type InventionStep = typeof INVENTION_STEPS[number];

/** Demo invention stream — shows a function being created step by step */
export const DEMO_INVENTION = {
  name: "content-quality-scorer",
  currentStep: "tasks" as InventionStep,
  steps: {
    essay: {
      status: "complete" as const,
      text: "This function evaluates content quality by examining coherence, originality, and technical accuracy. It composes three scalar sub-judgments: a coherence scorer that checks logical flow and argument structure, an originality detector that identifies derivative content, and a technical accuracy verifier that cross-references claims against known facts. The final score blends these perspectives using learned weights that adapt to the content domain.",
    },
    input_schema: {
      status: "complete" as const,
      text: JSON.stringify({
        type: "object",
        properties: {
          content: { type: "string", description: "The content to evaluate" },
          domain: { type: "string", description: "Content domain (e.g., 'technical', 'creative', 'academic')" },
        },
        required: ["content"],
      }, null, 2),
    },
    essay_tasks: {
      status: "complete" as const,
      text: "Task 1: Coherence — assess logical flow, argument structure, and internal consistency.\nTask 2: Originality — detect derivative content, clichés, and boilerplate patterns.\nTask 3: Technical accuracy — verify factual claims and technical statements.",
    },
    tasks: {
      status: "streaming" as const,
      text: "Generating task definitions for coherence scorer... The coherence task uses a vector completion with 5 response levels (excellent, good, adequate, poor, incoherent). Each agent evaluates the",
    },
    description: {
      status: "pending" as const,
      text: "",
    },
  },
};

/** Demo vector completion — single task, prompt + responses → votes */
export const DEMO_VECTOR_COMPLETION = {
  prompt: "Is the following text written by a human or generated by AI?\n\n\"The paradigm shift in quantum computing necessitates a fundamental reassessment of our cryptographic infrastructure.\"",
  responses: [
    "Human-written",
    "AI-generated",
    "Uncertain",
  ],
  votes: [
    { model: "llm-001", vote: [0.15, 0.75, 0.10], weight: 1.2 },
    { model: "llm-002", vote: [0.20, 0.70, 0.10], weight: 1.0 },
    { model: "llm-003", vote: [0.10, 0.80, 0.10], weight: 0.8 },
    { model: "llm-004", vote: [0.25, 0.60, 0.15], weight: 1.1 },
    { model: "llm-005", vote: [0.30, 0.55, 0.15], weight: 0.6 },
  ],
  scores: [0.19, 0.69, 0.12],
  weights: [1.2, 1.0, 0.8, 1.1, 0.6],
};
