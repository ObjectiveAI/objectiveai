"use client";

import { useState, useCallback, useEffect, useRef } from "react";
import { JudgmentStack } from "@/components/JudgmentStack";
import type { FunctionExecution } from "@/components/JudgmentStack";
import { InventionStream } from "@/components/InventionStream";
import { AgentChat } from "@/components/AgentChat";
import type { ChatMessage } from "@/components/AgentChat";
import { useViewerEvents } from "@/lib/useViewerEvents";
import type { ViewerEntry as LiveEntry } from "@/lib/useViewerEvents";
import type { FunctionDefinition } from "@/lib/functions/types";
import type { ProfileMeta, ProfileLlm } from "@/lib/profiles/types";
import type { InventionStep } from "@/lib/demo-data";
import type {
  FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  FunctionsExecutionsResponseStreamingTaskChunk,
  AgentCompletionsResponseStreamingAgentCompletionChunk,
  FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk,
  LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk,
} from "objectiveai";
import {
  DEMO_EXECUTION_COMPLETE,
  DEMO_MODEL_NAMES,
  DEMO_INVENTION,
} from "@/lib/demo-data";
import styles from "./viewer.module.css";

/* ══════════════════════════════════════════════════════════════
   SECTION 1 — SDK chunk → JudgmentStack adapter
   ══════════════════════════════════════════════════════════════ */

function extractOutput(raw: unknown): number | number[] | undefined {
  if (typeof raw === "number") return raw;
  if (Array.isArray(raw)) return raw as number[];
  if (raw && typeof raw === "object") {
    const obj = raw as Record<string, unknown>;
    if (typeof obj.value === "number") return obj.value;
    if (Array.isArray(obj.value)) return obj.value as number[];
  }
  return undefined;
}

function chunkToJudgmentExecution(
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
): FunctionExecution {
  return {
    output: extractOutput((chunk as Record<string, unknown>).output),
    tasks: chunk.tasks.map((task: FunctionsExecutionsResponseStreamingTaskChunk, i: number) => {
      if ("votes" in task && "scores" in task) {
        const vcTask = task as {
          votes: Array<{
            agent: string;
            vote: number[];
            weight: number;
            from_cache?: boolean;
            from_rng?: boolean;
          }>;
          scores: number[];
          completions?: Array<Record<string, unknown>>;
          task_path?: number[];
        };
        if (!Array.isArray(vcTask.votes) || !Array.isArray(vcTask.scores))
          return { task_path: [i] };
        return {
          task_path: vcTask.task_path ?? [i],
          votes: vcTask.votes.map((v) => ({
            model: v.agent,
            vote: v.vote,
            weight: v.weight,
            from_cache: v.from_cache,
            from_rng: v.from_rng,
          })),
          scores: vcTask.scores,
          completions: vcTask.completions,
        };
      }
      return { task_path: [i] };
    }),
  };
}

function syntheticDefinition(
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
): FunctionDefinition {
  return {
    type: "scalar.function",
    tasks: chunk.tasks.map((task: FunctionsExecutionsResponseStreamingTaskChunk) => {
      const obj = task as Record<string, unknown>;
      const objStr = typeof obj.object === "string" ? (obj.object as string) : "";
      if (objStr.includes("vector_completion") || ("votes" in task && "scores" in task)) {
        return { type: "vector.completion" };
      }
      if (objStr.includes("function") || "tasks" in task) {
        return { type: "scalar.function" };
      }
      return { type: "vector.completion" };
    }),
  };
}

function executionLabel(entry: LiveEntry): string {
  const req = entry.request;
  if (req.function && typeof req.function === "object") {
    const fn = req.function as Record<string, unknown>;
    if (fn.owner && fn.repository) return `${fn.owner}/${fn.repository}`;
  }
  if (typeof req.function === "string") return req.function;
  return entry.id.slice(0, 12);
}

function inventionLabel(entry: LiveEntry): string {
  const req = entry.request;
  if (typeof req.name === "string") return req.name;
  return entry.id.slice(0, 12);
}

function agentLabel(entry: LiveEntry): string {
  if (entry.kind === "agent-completion" && entry.chunk) {
    for (const msg of (entry.chunk.messages ?? [])) {
      if (msg.role === "assistant") {
        const a = msg as { model?: string };
        if (a.model) return a.model;
      }
    }
  }
  return entry.id.slice(0, 12);
}

function entryLabel(entry: LiveEntry): string {
  switch (entry.kind) {
    case "execution":
      return executionLabel(entry);
    case "invention":
      return inventionLabel(entry);
    case "agent-completion":
      return agentLabel(entry);
    case "laboratory":
      return entry.id.slice(0, 12);
  }
}

function entryState(entry: LiveEntry): string {
  if (entry.error) return "error";
  if (!entry.chunk) return "pending";

  if (entry.kind === "agent-completion") {
    const msgs = entry.chunk.messages ?? [];
    const hasFinish = msgs.some(
      (m) => m.role === "assistant" && (m as Record<string, unknown>).finish_reason,
    );
    return hasFinish ? "complete" : "streaming";
  }

  if (entry.kind === "execution") {
    const obj = (entry.chunk as Record<string, unknown>).object;
    if (typeof obj === "string" && obj.includes("chunk")) return "streaming";
    return "complete";
  }

  return "streaming";
}

/* ── Agent completion chunk → AgentChat adapter ── */

function extractModel(chunk: AgentCompletionsResponseStreamingAgentCompletionChunk): string | undefined {
  for (const msg of chunk.messages ?? []) {
    if (msg.role === "assistant") {
      const a = msg as Record<string, unknown>;
      if (typeof a.model === "string") return a.model;
    }
  }
  return undefined;
}

function chunkToChat(
  chunk: AgentCompletionsResponseStreamingAgentCompletionChunk,
  request: Record<string, unknown>,
): ChatMessage[] {
  const messages: ChatMessage[] = [];
  const reqMsgs = Array.isArray(request.messages) ? request.messages as Array<Record<string, unknown>> : [];
  for (const m of reqMsgs) {
    const role = m.role as string;
    const content = typeof m.content === "string" ? m.content : JSON.stringify(m.content);
    if (role === "system" || role === "developer") {
      messages.push({ role: "system", content });
    } else if (role === "user") {
      messages.push({ role: "user", content });
    }
  }
  for (const msg of chunk.messages ?? []) {
    if (msg.role === "assistant") {
      const a = msg as Record<string, unknown>;
      messages.push({
        role: "assistant",
        content: typeof a.content === "string" ? a.content : "",
        model: typeof a.model === "string" ? a.model : undefined,
        reasoning: typeof a.reasoning === "string" ? a.reasoning : undefined,
        finish_reason: typeof a.finish_reason === "string" ? a.finish_reason : undefined,
      } as ChatMessage);
    } else if (msg.role === "tool") {
      const t = msg as Record<string, unknown>;
      messages.push({
        role: "tool",
        tool_call_id: typeof t.tool_call_id === "string" ? t.tool_call_id : "",
        content: typeof t.content === "string" ? t.content : JSON.stringify(t.content),
      });
    }
  }
  return messages;
}

/* ── Live invention renderer ── */

function LiveInventionView({
  chunk,
}: {
  chunk: FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk;
}) {
  const inventions = (chunk as Record<string, unknown>).inventions as Array<Record<string, unknown>> | undefined;
  if (!inventions || inventions.length === 0) {
    return <div className={styles.emptyState}>waiting for invention data…</div>;
  }
  return (
    <div className={styles.inventionLive}>
      {inventions.map((inv, i) => {
        const state = inv.state as Record<string, unknown> | undefined;
        const name = (state && typeof state.name === "string") ? state.name : `invention #${inv.index ?? i}`;
        const completions = Array.isArray(inv.completions) ? inv.completions as Array<Record<string, unknown>> : [];
        const invError = inv.error as { code: number; message: unknown } | undefined;
        return (
          <div key={inv.index as number ?? i} className={styles.inventionStep}>
            <div className={styles.inventionStepHeader}>
              <span className={styles.entryDot} style={{ background: invError ? "var(--error)" : completions.length > 0 ? "var(--copper-hot)" : "var(--copper-mid)" }} />
              <span className={styles.inventionStepName}>{name}</span>
              {completions.length > 0 && (
                <span className={styles.inventionStepMeta}>{completions.length} completions</span>
              )}
            </div>
            {completions.map((comp, ci) => {
              const msgs = Array.isArray(comp.messages) ? comp.messages as Array<Record<string, unknown>> : [];
              const assistantMsg = msgs.find((m) => m.role === "assistant");
              const content = assistantMsg && typeof assistantMsg.content === "string" ? assistantMsg.content : "";
              const model = assistantMsg && typeof assistantMsg.model === "string" ? assistantMsg.model : "";
              return (
                <div key={ci} className={styles.inventionCompletion}>
                  {model && <span className={styles.inventionCompletionModel}>{model}</span>}
                  <span className={styles.inventionCompletionText}>{content || "…"}</span>
                </div>
              );
            })}
            {invError && (
              <div className={styles.inventionError}>
                Error {invError.code}: {JSON.stringify(invError.message)}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

/* ── Live laboratory renderer ── */

function LiveLaboratoryView({
  chunk,
}: {
  chunk: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk;
}) {
  const raw = chunk as Record<string, unknown>;
  const executions = Array.isArray(raw.executions) ? raw.executions as Array<Record<string, unknown>> : [];
  if (executions.length === 0) {
    return (
      <div className={styles.jsonView}>
        <pre>{JSON.stringify(chunk, null, 2)}</pre>
      </div>
    );
  }
  return (
    <div className={styles.labLive}>
      {executions.map((exec, i) => {
        const fn = typeof exec.function === "string" ? exec.function : `execution #${i}`;
        const output = extractOutput(exec.output);
        return (
          <div key={i} className={styles.labEntry}>
            <span className={styles.labEntryName}>{fn}</span>
            {output != null && (
              <span className={styles.labEntryScore}>
                {typeof output === "number" ? (output * 100).toFixed(1) + "%" : JSON.stringify(output)}
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}

/* ══════════════════════════════════════════════════════════════
   SECTION 2 — Demo mode types + data (fallback when no CLI)
   ══════════════════════════════════════════════════════════════ */

interface StepData {
  status: "complete" | "streaming" | "pending";
  text: string;
}

type DemoEntry =
  | {
      kind: "execution";
      id: string;
      label: string;
      state: "pending" | "streaming" | "complete" | "error";
      execution: FunctionExecution | null;
      definition: FunctionDefinition | null;
      profile: ProfileMeta | null;
    }
  | {
      kind: "invention";
      id: string;
      label: string;
      state: "idle" | "streaming" | "done" | "error";
      currentStep: InventionStep | null;
      steps: Record<InventionStep, StepData>;
    }
  | {
      kind: "agent-completion";
      id: string;
      label: string;
      state: "streaming" | "complete" | "error";
      messages: ChatMessage[];
      model?: string;
    }
  | {
      kind: "laboratory";
      id: string;
      label: string;
      state: "streaming" | "complete" | "error";
      data: { executions: LabExecution[] };
    };

interface LabExecution {
  model: string;
  score: number;
  latency_ms: number;
}

const DEFINITION: FunctionDefinition = {
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

const PROFILE: ProfileMeta = {
  remote: "ObjectiveAI/profile-standard",
  owner: "ObjectiveAI",
  repository: "profile-standard",
  commit: "abc123",
  name: "profile-standard",
  description: "Standard swarm with frontier and mid-tier models",
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
      { llm: { model: "openai/gpt-4o", outputMode: "log_probs", topLogprobs: 5, temperature: null, reasoning: null, count: 1, fallbacks: [] } as ProfileLlm, weight: 1.2 },
      { llm: { model: "anthropic/claude-3.5-sonnet", outputMode: "log_probs", topLogprobs: 5, temperature: null, reasoning: null, count: 1, fallbacks: [] } as ProfileLlm, weight: 1.0 },
    ],
    mid: [
      { llm: { model: "google/gemini-2.5-flash", outputMode: "log_probs", topLogprobs: 5, temperature: null, reasoning: null, count: 1, fallbacks: [] } as ProfileLlm, weight: 0.8 },
      { llm: { model: "meta/llama-3.3-70b", outputMode: "log_probs", topLogprobs: null, temperature: null, reasoning: null, count: 1, fallbacks: [] } as ProfileLlm, weight: 1.1 },
    ],
    budget: [
      { llm: { model: "deepseek/deepseek-v3", outputMode: "default", topLogprobs: null, temperature: null, reasoning: null, count: 1, fallbacks: [] } as ProfileLlm, weight: 0.6 },
    ],
  },
};

const DEMO_AGENT_MESSAGES: ChatMessage[] = [
  { role: "system", content: "You are an ObjectiveAI agent. Evaluate the given code and provide a quality assessment." },
  { role: "user", content: "function fibonacci(n) {\n  if (n <= 1) return n;\n  return fibonacci(n - 1) + fibonacci(n - 2);\n}" },
  { role: "assistant", content: "This is a classic recursive Fibonacci implementation. While correct, it has exponential O(2^n) time complexity due to redundant sub-problem computation. Each call spawns two more calls, leading to an exponential call tree.\n\nFor production use, consider:\n\n• Memoized: O(n) time, O(n) space\n• Iterative: O(n) time, O(1) space\n• Matrix exponentiation: O(log n) time\n\nThe code is readable and well-named but unsuitable for n > ~40 without optimization.", model: "anthropic/claude-3.5-sonnet", finish_reason: "stop" },
];

const DEMO_LAB_DATA = {
  executions: [
    { model: "openai/gpt-4o", score: 0.82, latency_ms: 1240 },
    { model: "anthropic/claude-3.5-sonnet", score: 0.79, latency_ms: 980 },
    { model: "google/gemini-2.5-flash", score: 0.71, latency_ms: 540 },
    { model: "meta/llama-3.3-70b", score: 0.68, latency_ms: 1450 },
    { model: "deepseek/deepseek-v3", score: 0.63, latency_ms: 820 },
  ],
};

/* ══════════════════════════════════════════════════════════════
   SECTION 3 — Demo playback hook
   ══════════════════════════════════════════════════════════════ */

function useDemoPlayback() {
  const [entries, setEntries] = useState<DemoEntry[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [playing, setPlaying] = useState(false);
  const timeoutsRef = useRef<ReturnType<typeof setTimeout>[]>([]);

  const clearTimeouts = useCallback(() => {
    timeoutsRef.current.forEach(clearTimeout);
    timeoutsRef.current = [];
  }, []);

  const schedule = useCallback((fn: () => void, delay: number) => {
    const id = setTimeout(fn, delay);
    timeoutsRef.current.push(id);
  }, []);

  const play = useCallback(() => {
    clearTimeouts();
    setEntries([]);
    setSelected(null);
    setPlaying(true);

    schedule(() => {
      const execEntry: DemoEntry = {
        kind: "execution",
        id: "exec-a7f3b2c1",
        label: "ObjectiveAI/is-code",
        state: "streaming",
        execution: {
          id: "exec-a7f3b2c1",
          function: "ObjectiveAI/is-code",
          profile: "ObjectiveAI/profile-standard",
          tasks: DEMO_EXECUTION_COMPLETE.tasks!.map((t) => ({
            task_path: t.task_path,
            votes: [],
            scores: [],
          })),
        },
        definition: DEFINITION,
        profile: PROFILE,
      };
      setEntries([execEntry]);
      setSelected("exec-a7f3b2c1");
    }, 400);

    const allVotes = DEMO_EXECUTION_COMPLETE.tasks!.flatMap((task, ti) =>
      (task.votes ?? []).map((v) => ({ taskIndex: ti, vote: v })),
    );

    allVotes.forEach((item, i) => {
      schedule(() => {
        setEntries((prev) =>
          prev.map((e) => {
            if (e.kind !== "execution" || e.id !== "exec-a7f3b2c1") return e;
            const exec = e.execution;
            if (!exec?.tasks) return e;
            const tasks = exec.tasks.map((task, ti) => {
              if (ti !== item.taskIndex) return task;
              const votes = [...(task.votes ?? []), item.vote];
              const totalW = votes.reduce((s, v) => s + v.weight, 0);
              const numR = item.vote.vote.length;
              const scores = Array.from({ length: numR }, (_, ri) =>
                totalW > 0
                  ? votes.reduce((s, v) => s + v.vote[ri] * v.weight, 0) / totalW
                  : 0,
              );
              return { ...task, votes, scores };
            });
            return { ...e, execution: { ...exec, tasks } };
          }),
        );
      }, 800 + i * 400);
    });

    const execDoneDelay = 800 + allVotes.length * 400 + 300;

    schedule(() => {
      setEntries((prev) =>
        prev.map((e) => {
          if (e.kind !== "execution" || e.id !== "exec-a7f3b2c1") return e;
          return { ...e, state: "complete" as const, execution: DEMO_EXECUTION_COMPLETE };
        }),
      );
    }, execDoneDelay);

    schedule(() => {
      setEntries((prev) => [
        ...prev,
        {
          kind: "agent-completion",
          id: "ac-c8f1a2d3",
          label: "claude-3.5-sonnet",
          state: "complete" as const,
          messages: DEMO_AGENT_MESSAGES,
          model: "anthropic/claude-3.5-sonnet",
        },
      ]);
    }, execDoneDelay + 600);

    schedule(() => {
      setEntries((prev) => [
        ...prev,
        {
          kind: "invention",
          id: "inv-e5f6a7b8",
          label: "content-quality-scorer",
          state: "streaming" as const,
          currentStep: DEMO_INVENTION.currentStep,
          steps: DEMO_INVENTION.steps,
        },
      ]);
    }, execDoneDelay + 1200);

    schedule(() => {
      setEntries((prev) => [
        ...prev,
        {
          kind: "laboratory",
          id: "lab-d4e5f6a7",
          label: "model-comparison",
          state: "complete" as const,
          data: DEMO_LAB_DATA,
        },
      ]);
      setPlaying(false);
    }, execDoneDelay + 1800);
  }, [clearTimeouts, schedule]);

  useEffect(() => {
    play();
    return clearTimeouts;
  }, [play, clearTimeouts]);

  return { entries, selected, setSelected, playing, play };
}

/* ══════════════════════════════════════════════════════════════
   SECTION 4 — Shared UI helpers
   ══════════════════════════════════════════════════════════════ */

function dotColor(state: string): string {
  if (state === "complete" || state === "done") return "var(--copper-hot)";
  if (state === "streaming") return "var(--copper-mid)";
  if (state === "error") return "var(--error)";
  return "var(--node-border)";
}

function kindBadge(kind: string): string {
  switch (kind) {
    case "execution": return "exec";
    case "invention": return "inv";
    case "agent-completion": return "ac";
    case "laboratory": return "lab";
    default: return kind;
  }
}

/* ══════════════════════════════════════════════════════════════
   SECTION 5 — Page
   ══════════════════════════════════════════════════════════════ */

type ViewerMode = "live" | "demo";

export default function Viewer() {
  const [mode, setMode] = useState<ViewerMode>("demo");
  const [userChoseDemo, setUserChoseDemo] = useState(false);

  const live = useViewerEvents("/api/viewer/events");
  const demo = useDemoPlayback();

  const hasLiveData = live.entries.length > 0;
  const effectiveMode = userChoseDemo ? "demo" : hasLiveData ? "live" : mode;

  const [selected, setSelected] = useState<string | null>(null);

  const currentSelected = effectiveMode === "live" ? selected : demo.selected;
  const setCurrentSelected = effectiveMode === "live" ? setSelected : demo.setSelected;

  const connected = effectiveMode === "live"
    ? live.connectionState === "connected"
    : true;

  const connectionLabel = effectiveMode === "live"
    ? live.connectionState
    : "demo";

  return (
    <div className={styles.viewer}>
      {/* Title bar */}
      <div className={styles.titleBar}>
        <div className={styles.titleBarDots}>
          <span className={`${styles.windowDot} ${styles.dotClose}`} />
          <span className={`${styles.windowDot} ${styles.dotMinimize}`} />
          <span className={`${styles.windowDot} ${styles.dotMaximize}`} />
        </div>
        <span className={styles.titleBarText}>objectiveai viewer</span>
        <span className={styles.connectionStatus}>
          <span
            className={styles.connectionDot}
            style={{ background: connected ? "var(--copper-hot)" : "var(--copper-dim)" }}
          />
          {connectionLabel}
        </span>
      </div>

      {/* Body */}
      <div className={styles.body}>
        {/* Sidebar */}
        <div className={styles.sidebar}>
          <span className={styles.sidebarLabel}>entries</span>

          {effectiveMode === "live"
            ? live.entries.map((entry) => (
                <div
                  key={entry.id}
                  className={`${styles.entryItem}${currentSelected === entry.id ? ` ${styles.entryItemSelected}` : ""}`}
                  onClick={() => setCurrentSelected(entry.id)}
                >
                  <span className={styles.entryDot} style={{ background: dotColor(entryState(entry)) }} />
                  <span className={styles.entryKind}>{kindBadge(entry.kind)}</span>
                  <span className={styles.entryLabel}>{entryLabel(entry)}</span>
                </div>
              ))
            : demo.entries.map((entry) => (
                <div
                  key={entry.id}
                  className={`${styles.entryItem}${currentSelected === entry.id ? ` ${styles.entryItemSelected}` : ""}`}
                  onClick={() => setCurrentSelected(entry.id)}
                >
                  <span className={styles.entryDot} style={{ background: dotColor(entry.state) }} />
                  <span className={styles.entryKind}>{kindBadge(entry.kind)}</span>
                  <span className={styles.entryLabel}>{entry.label}</span>
                </div>
              ))}

          {(effectiveMode === "live" ? live.entries : demo.entries).length === 0 && (
            <div className={styles.entryItem} style={{ cursor: "default" }}>
              <span className={styles.entryLabel} style={{ fontStyle: "italic" }}>
                waiting for events…
              </span>
            </div>
          )}
        </div>

        {/* Main content */}
        <div className={styles.main}>
          {effectiveMode === "live"
            ? <LiveContent entries={live.entries} selected={currentSelected} />
            : <DemoContent entries={demo.entries} selected={currentSelected} />}
        </div>
      </div>

      {/* Status bar */}
      <div className={styles.statusBar}>
        <span>
          {(effectiveMode === "live" ? live.entries : demo.entries).length}{" "}
          {(effectiveMode === "live" ? live.entries : demo.entries).length === 1 ? "entry" : "entries"}
        </span>
        {effectiveMode === "live" && <span>· live</span>}
        {effectiveMode === "demo" && demo.playing && <span>· streaming</span>}

        <button
          className={styles.replayButton}
          onClick={() => {
            setUserChoseDemo(true);
            setMode("demo");
            demo.play();
          }}
          disabled={demo.playing}
        >
          {demo.playing ? "playing…" : "replay demo"}
        </button>
      </div>
    </div>
  );
}

/* ══════════════════════════════════════════════════════════════
   SECTION 6 — Live content renderer (real SDK chunks)
   ══════════════════════════════════════════════════════════════ */

function LiveContent({ entries, selected }: { entries: LiveEntry[]; selected: string | null }) {
  const entry = entries.find((e) => e.id === selected);

  if (!entry) {
    return (
      <div className={styles.emptyState}>
        <div className={styles.emptyPulse} />
        {entries.length === 0 ? "waiting for events from cli" : "select an entry"}
      </div>
    );
  }

  return (
    <div className={styles.mainInner}>
      <div className={styles.entryHeader}>
        <span className={styles.entryDot} style={{ background: dotColor(entryState(entry)) }} />
        <span className={styles.entryTitle}>{entryLabel(entry)}</span>
        <span className={styles.entryBadge}>{entry.kind.replace("-", " ")}</span>
        <span className={styles.entryId}>{entry.id}</span>
      </div>

      {entry.kind === "execution" && entry.chunk && (
        <JudgmentStack
          definition={syntheticDefinition(entry.chunk)}
          execution={chunkToJudgmentExecution(entry.chunk)}
        />
      )}

      {entry.kind === "execution" && !entry.chunk && !entry.error && (
        <div className={styles.emptyState}>awaiting execution data</div>
      )}

      {entry.kind === "agent-completion" && entry.chunk && (
        <AgentChat
          messages={chunkToChat(entry.chunk, entry.request)}
          model={extractModel(entry.chunk)}
          status={entryState(entry) === "complete" ? "complete" : entryState(entry) === "error" ? "error" : "streaming"}
        />
      )}

      {entry.kind === "invention" && entry.chunk && (
        <LiveInventionView chunk={entry.chunk} />
      )}

      {entry.kind === "laboratory" && entry.chunk && (
        <LiveLaboratoryView chunk={entry.chunk} />
      )}

      {entry.error && (
        <div className={styles.jsonView} style={{ color: "var(--error)" }}>
          Error {entry.error.code}: {JSON.stringify(entry.error.message)}
        </div>
      )}

      {!entry.chunk && !entry.error && entry.kind !== "execution" && (
        <div className={styles.emptyState}>waiting for data…</div>
      )}
    </div>
  );
}

/* ══════════════════════════════════════════════════════════════
   SECTION 7 — Demo content renderer (simulated data)
   ══════════════════════════════════════════════════════════════ */

function DemoContent({ entries, selected }: { entries: DemoEntry[]; selected: string | null }) {
  const entry = entries.find((e) => e.id === selected);

  if (!entry) {
    return (
      <div className={styles.emptyState}>
        <div className={styles.emptyPulse} />
        {entries.length === 0 ? "waiting for events from cli" : "select an entry"}
      </div>
    );
  }

  return (
    <div className={styles.mainInner}>
      <div className={styles.entryHeader}>
        <span className={styles.entryDot} style={{ background: dotColor(entry.state) }} />
        <span className={styles.entryTitle}>{entry.label}</span>
        <span className={styles.entryBadge}>{entry.kind.replace("-", " ")}</span>
        <span className={styles.entryId}>{entry.id}</span>
      </div>

      {entry.kind === "execution" && (
        entry.execution ? (
          <JudgmentStack
            definition={entry.definition}
            execution={entry.execution}
            profile={entry.profile}
            modelNames={DEMO_MODEL_NAMES}
          />
        ) : (
          <div className={styles.emptyState}>awaiting execution data</div>
        )
      )}

      {entry.kind === "invention" && (
        <InventionStream
          name={entry.label}
          currentStep={entry.currentStep}
          steps={entry.steps}
          state={entry.state}
        />
      )}

      {entry.kind === "agent-completion" && (
        <AgentChat
          messages={entry.messages}
          model={entry.model}
          status={entry.state}
        />
      )}

      {entry.kind === "laboratory" && (
        <LabView executions={entry.data.executions} />
      )}
    </div>
  );
}

/* ══════════════════════════════════════════════════════════════
   SECTION 8 — Laboratory structured view
   ══════════════════════════════════════════════════════════════ */

function LabView({ executions }: { executions: LabExecution[] }) {
  const maxScore = Math.max(...executions.map((e) => e.score), 0.01);

  return (
    <div className={styles.labGrid}>
      {executions.map((exec, i) => (
        <div key={i} className={styles.labRow}>
          <span className={styles.labModel}>{exec.model}</span>
          <div
            className={styles.labBar}
            style={{
              width: `${(exec.score / maxScore) * 120}px`,
              background: exec.score === maxScore ? "var(--copper-hot)" : "var(--copper-dim)",
            }}
          />
          <span className={styles.labScore}>{(exec.score * 100).toFixed(1)}%</span>
          <span className={styles.labLatency}>{exec.latency_ms}ms</span>
        </div>
      ))}
    </div>
  );
}
