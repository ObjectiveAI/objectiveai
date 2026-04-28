"use client";

import { useState, useCallback } from "react";
import { JudgmentStack } from "@/components/JudgmentStack";
import type { FunctionExecution } from "@/components/JudgmentStack";
import { InventionStream } from "@/components/InventionStream";
import {
  DEMO_EXECUTION_COMPLETE,
  DEMO_MODEL_NAMES,
  DEMO_INVENTION,
} from "@/lib/demo-data";
import type { FunctionDefinition } from "@/lib/functions/types";
import type { ProfileMeta, ProfileLlm } from "@/lib/profiles/types";
import styles from "./viewer-preview.module.css";

/* ── Shared definition + profile (same as demo page) ── */

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

/* ── Streaming simulation ── */

const FULL_EXEC = DEMO_EXECUTION_COMPLETE;

function useStreamingExecution() {
  const [exec, setExec] = useState<FunctionExecution | null>(null);
  const [phase, setPhase] = useState<"idle" | "streaming" | "done">("idle");

  const replay = useCallback(() => {
    setExec(null);
    setPhase("streaming");

    const allVotes = FULL_EXEC.tasks!.flatMap((t, ti) =>
      (t.votes ?? []).map((v) => ({ taskIndex: ti, vote: v }))
    );

    setTimeout(() => {
      setExec({
        id: FULL_EXEC.id,
        function: FULL_EXEC.function,
        profile: FULL_EXEC.profile,
        tasks: FULL_EXEC.tasks!.map((t) => ({
          task_path: t.task_path,
          votes: [],
          scores: [],
        })),
      });
    }, 300);

    allVotes.forEach((item, i) => {
      setTimeout(() => {
        setExec((prev) => {
          if (!prev?.tasks) return prev;
          const tasks = prev.tasks.map((t, ti) => {
            if (ti !== item.taskIndex) return t;
            const votes = [...(t.votes ?? []), item.vote];
            const totalW = votes.reduce((s, v) => s + v.weight, 0);
            const numR = item.vote.vote.length;
            const scores = Array.from({ length: numR }, (_, ri) =>
              totalW > 0 ? votes.reduce((s, v) => s + v.vote[ri] * v.weight, 0) / totalW : 0
            );
            return { ...t, votes, scores };
          });
          return { ...prev, tasks };
        });
      }, 700 + i * 500);
    });

    setTimeout(() => {
      setExec((prev) => ({
        ...prev,
        output: FULL_EXEC.output,
        reasoning: FULL_EXEC.reasoning,
      }));
      setPhase("done");
    }, 700 + allVotes.length * 500 + 400);
  }, []);

  return { exec, phase, replay };
}

/* ── Page ── */

export default function ViewerPreview() {
  const { exec, phase, replay } = useStreamingExecution();

  return (
    <div className={styles.viewer}>
      {/* Title bar simulating Tauri window chrome */}
      <div className={styles.titleBar}>
        <span className={styles.titleBarDot} data-color="close" />
        <span className={styles.titleBarDot} data-color="minimize" />
        <span className={styles.titleBarDot} data-color="maximize" />
        <span className={styles.titleBarText}>objectiveai viewer</span>
        <span className={styles.connectionStatus}>
          <span className={styles.connectionDot} />
          local
        </span>
      </div>

      <div className={styles.content}>
        {/* Execution panel */}
        <section className={styles.section}>
          <div className={styles.sectionHeader}>
            <h2 className={styles.sectionTitle}>execution</h2>
            <button
              onClick={replay}
              disabled={phase === "streaming"}
              className={styles.actionButton}
            >
              {phase === "idle" ? "execute" : phase === "streaming" ? "running\u2026" : "re-run"}
            </button>
          </div>
          {exec ? (
            <JudgmentStack
              definition={DEFINITION}
              execution={exec}
              profile={PROFILE}
              modelNames={DEMO_MODEL_NAMES}
            />
          ) : (
            <div className={styles.empty}>
              awaiting execution
            </div>
          )}
        </section>

        {/* Invention panel */}
        <section className={styles.section}>
          <div className={styles.sectionHeader}>
            <h2 className={styles.sectionTitle}>invention</h2>
          </div>
          <InventionStream
            name={DEMO_INVENTION.name}
            currentStep={DEMO_INVENTION.currentStep}
            steps={DEMO_INVENTION.steps}
            state="streaming"
          />
        </section>
      </div>
    </div>
  );
}
