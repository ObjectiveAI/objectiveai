"use client";

import { useMemo } from "react";
import type {
  InputFunctionExecution,
  InputFunctionDefinition,
  InputProfile,
} from "@objectiveai/function-tree/core";

/* eslint-disable @typescript-eslint/no-explicit-any */

// Prototype-local type for tasks (union of VectorCompletion | FunctionExecution)
// Using loose type to avoid union narrowing boilerplate in prototype components.
interface ProtoTask {
  index?: number;
  task_index?: number;
  task_path?: number[];
  votes?: Array<{ model: string; vote: number[]; weight: number; from_cache?: boolean; from_rng?: boolean }>;
  completions?: any[];
  scores?: number[];
  error?: { message?: string } | null;
  tasks?: ProtoTask[];
  output?: number | number[];
}

// ---------------------------------------------------------------------------
// Shared types for prototypes
// ---------------------------------------------------------------------------

interface PrototypeProps {
  execution: InputFunctionExecution;
  definition?: InputFunctionDefinition | null;
  profile?: InputProfile | null;
  modelNames?: Record<string, string>;
  responseLabels?: Record<string, string[]>;
}

// ---------------------------------------------------------------------------
// Shared utilities
// ---------------------------------------------------------------------------

const mono: React.CSSProperties = {
  fontFamily:
    '"JetBrains Mono", "Fira Code", "SF Mono", "Cascadia Code", monospace',
};

function scoreBar(score: number, width: number = 20): string {
  const filled = Math.round(score * width);
  return "█".repeat(filled) + "░".repeat(width - filled);
}

function scoreColor(score: number): string {
  if (score >= 0.5) return "#f59e0b";
  if (score >= 0.3) return "#d97706";
  if (score >= 0.15) return "#b45309";
  return "#92400e";
}

function stateColor(state: string): string {
  switch (state) {
    case "complete": return "#f59e0b";
    case "streaming": return "#d97706";
    case "error": return "#b91c1c";
    default: return "#292524";
  }
}

function execState(exec: InputFunctionExecution): string {
  if (exec.error) return "error";
  if (exec.output !== undefined && exec.output !== null) return "complete";
  if (exec.tasks && exec.tasks.length > 0) return "streaming";
  return "pending";
}

function taskState(task: any): string {
  if (task.error) return "error";
  if (task.scores && task.scores.length > 0) return "complete";
  if (task.completions && task.completions.length > 0) return "streaming";
  return "pending";
}

// ---------------------------------------------------------------------------
// PROTOTYPE 1: Terminal/TUI Stacked Panel
//
// Dense monospace layout. Function header → task sections → LLM rows.
// Box-drawing characters. Everything scannable top to bottom.
// ---------------------------------------------------------------------------

export function TerminalPanel({ execution, definition, profile, modelNames, responseLabels }: PrototypeProps) {
  const output = execution.output as number[] | number | null;
  const state = execState(execution);
  const reasoning = execution.reasoning?.choices?.[0]?.message?.content ?? null;

  return (
    <div
      style={{
        ...mono,
        fontSize: 12,
        lineHeight: "20px",
        color: "#d6d3d1",
        background: "#141210",
        border: "1px solid #292524",
        borderRadius: 4,
        padding: 0,
        overflow: "hidden",
      }}
    >
      {/* Function header */}
      <div
        style={{
          padding: "12px 16px",
          borderBottom: "1px solid #292524",
          display: "flex",
          alignItems: "baseline",
          gap: 12,
        }}
      >
        <span style={{ color: stateColor(state), fontSize: 8, marginRight: 4 }}>●</span>
        <span style={{ color: "#d6d3d1", fontWeight: 700, fontSize: 14 }}>
          {execution.function?.split("/").pop() || "Function"}
        </span>
        {output !== null && (
          <span style={{ color: "#f59e0b", fontWeight: 700, fontSize: 14 }}>
            {Array.isArray(output)
              ? `#${output.indexOf(Math.max(...output)) + 1} · ${(Math.max(...output) * 100).toFixed(1)}%`
              : `${(output * 100).toFixed(1)}%`}
          </span>
        )}
        {execution.id && (
          <span style={{ color: "#78716c", fontSize: 10, marginLeft: "auto" }}>
            {execution.id}
          </span>
        )}
      </div>

      {/* Output distribution bar */}
      {Array.isArray(output) && (
        <div style={{ padding: "8px 16px", borderBottom: "1px solid #292524" }}>
          <div style={{ display: "flex", height: 6, borderRadius: 3, overflow: "hidden", gap: 1 }}>
            {output.map((score, i) => (
              <div
                key={i}
                style={{
                  flex: score,
                  background: scoreColor(score),
                  minWidth: score > 0 ? 2 : 0,
                }}
              />
            ))}
          </div>
        </div>
      )}

      {/* Reasoning */}
      {reasoning && (
        <div
          style={{
            padding: "6px 16px",
            borderBottom: "1px solid #292524",
            color: "#78716c",
            fontSize: 11,
            fontStyle: "italic",
          }}
        >
          <span style={{ color: "#d97706", fontStyle: "normal", fontWeight: 700, marginRight: 6 }}>R</span>
          {reasoning.length > 120 ? reasoning.slice(0, 117) + "…" : reasoning}
        </div>
      )}

      {/* Tasks */}
      {(execution.tasks as ProtoTask[])?.map((task, ti) => {
        const tState = taskState(task);
        const scores: number[] = task.scores ?? [];
        const pathKey = (task.task_path ?? [ti]).join(",");
        const labels = responseLabels?.[pathKey] ?? null;
        const prompt = definition?.tasks?.[ti]?.messages;
        const promptText = prompt
          ? (prompt.find((m: any) => m.role === "system") as any)?.content ??
            (prompt.find((m: any) => m.role === "user") as any)?.content ??
            null
          : null;
        const profileTask = profile?.tasks?.[ti];

        return (
          <div key={ti} style={{ borderBottom: "1px solid #292524" }}>
            {/* Task header */}
            <div
              style={{
                padding: "10px 16px 6px",
                display: "flex",
                alignItems: "baseline",
                gap: 8,
              }}
            >
              <span style={{ color: stateColor(tState), fontSize: 8 }}>●</span>
              <span style={{ fontWeight: 600, color: "#d6d3d1" }}>Task {ti}</span>
              {promptText && (
                <span style={{ color: "#78716c", fontSize: 11, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {typeof promptText === "string"
                    ? promptText.length > 80
                      ? promptText.slice(0, 77) + "…"
                      : promptText
                    : "[dynamic]"}
                </span>
              )}
              <span style={{ color: "#78716c", fontSize: 10 }}>
                {task.votes?.length ?? 0} LLMs
              </span>
            </div>

            {/* Score bars */}
            {scores.length > 0 && (
              <div style={{ padding: "4px 16px 8px" }}>
                {scores.map((score, si) => {
                  const label = labels?.[si] ?? `#${si + 1}`;
                  return (
                    <div
                      key={si}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 8,
                        height: 22,
                      }}
                    >
                      <span style={{ color: "#78716c", width: 80, textAlign: "right", fontSize: 11, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {label}
                      </span>
                      <div style={{ flex: 1, display: "flex", alignItems: "center", gap: 6 }}>
                        <div
                          style={{
                            flex: 1,
                            height: 8,
                            background: "#292524",
                            borderRadius: 4,
                            overflow: "hidden",
                          }}
                        >
                          <div
                            style={{
                              width: `${score * 100}%`,
                              height: "100%",
                              background: scoreColor(score),
                              borderRadius: 4,
                            }}
                          />
                        </div>
                        <span style={{ color: scoreColor(score), width: 52, textAlign: "right", fontSize: 11, fontWeight: 600 }}>
                          {(score * 100).toFixed(1)}%
                        </span>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}

            {/* LLM vote rows */}
            {task.votes && task.votes.length > 0 && (
              <div style={{ padding: "0 16px 10px" }}>
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11 }}>
                  <tbody>
                    {task.votes.map((vote, vi) => {
                      const name = modelNames?.[vote.model]
                        ? modelNames[vote.model]
                        : vote.model;
                      const shortName = name.includes("/") ? name.split("/").pop()! : name;
                      const llmDef = profileTask?.ensemble?.llms?.[vi];
                      const maxVote = Math.max(...vote.vote);

                      return (
                        <tr key={vi} style={{ borderTop: "1px solid #1c1917" }}>
                          {/* Model name */}
                          <td style={{ padding: "4px 0", color: "#d6d3d1", whiteSpace: "nowrap", width: 120 }}>
                            {shortName}
                          </td>
                          {/* Vote distribution mini bars */}
                          <td style={{ padding: "4px 8px" }}>
                            <div style={{ display: "flex", gap: 1, alignItems: "flex-end", height: 16 }}>
                              {vote.vote.map((v, i) => (
                                <div
                                  key={i}
                                  style={{
                                    width: 8,
                                    height: Math.max(2, v * 16),
                                    background: scoreColor(v),
                                    borderRadius: 1,
                                  }}
                                />
                              ))}
                            </div>
                          </td>
                          {/* Weight */}
                          <td style={{ padding: "4px 6px", color: "#78716c", whiteSpace: "nowrap" }}>
                            w={vote.weight.toFixed(2)}
                          </td>
                          {/* Badges */}
                          <td style={{ padding: "4px 0", whiteSpace: "nowrap" }}>
                            {vote.from_cache && (
                              <span style={{ color: "#292524", background: "#78716c", borderRadius: 2, padding: "1px 4px", fontSize: 9, marginRight: 4 }}>
                                CACHE
                              </span>
                            )}
                            {vote.from_rng && (
                              <span style={{ color: "#292524", background: "#d97706", borderRadius: 2, padding: "1px 4px", fontSize: 9, marginRight: 4 }}>
                                RNG
                              </span>
                            )}
                          </td>
                          {/* Output mode */}
                          <td style={{ padding: "4px 0", color: "#78716c", fontSize: 10, whiteSpace: "nowrap" }}>
                            {llmDef?.output_mode?.replace(/_/g, " ") ?? ""}
                            {llmDef?.top_logprobs ? ` · top${llmDef.top_logprobs}` : ""}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// PROTOTYPE 1B: Terminal Panel V2 — Refined
//
// Improvements over v1:
// - Weight bars (visual, not just text) on LLM rows
// - Profile task weight shown as border-left accent width
// - Vote distribution as horizontal stacked bars (wider, labeled)
// - Tighter vertical rhythm, better alignment grid
// ---------------------------------------------------------------------------

export function TerminalPanelV2({ execution, definition, profile, modelNames, responseLabels }: PrototypeProps) {
  const output = (execution.output !== undefined && execution.output !== null) ? execution.output as number[] | number : null;
  const state = execState(execution);
  const reasoning = execution.reasoning?.choices?.[0]?.message?.content ?? null;
  const tasks = (execution.tasks ?? []) as ProtoTask[];

  return (
    <div
      style={{
        ...mono,
        fontSize: 11,
        lineHeight: "18px",
        color: "#d6d3d1",
        background: "#141210",
        border: "1px solid #292524",
        borderRadius: 4,
        overflow: "hidden",
      }}
    >
      {/* ── Function header ── */}
      <div style={{ padding: "10px 16px", borderBottom: "1px solid #292524", display: "flex", alignItems: "center", gap: 10 }}>
        <span style={{ color: stateColor(state), fontSize: 7 }}>●</span>
        <span style={{ color: "#d6d3d1", fontWeight: 700, fontSize: 13 }}>
          {execution.function?.split("/").pop() || "Function"}
        </span>
        {output !== null ? (
          <span style={{ color: "#f59e0b", fontWeight: 700, fontSize: 13 }}>
            {Array.isArray(output)
              ? `#${output.indexOf(Math.max(...output)) + 1} · ${(Math.max(...output) * 100).toFixed(1)}%`
              : `${((output as number) * 100).toFixed(1)}%`}
          </span>
        ) : state === "streaming" ? (
          <span style={{ color: "#d97706", fontSize: 12 }}>running…</span>
        ) : null}
        <span style={{ flex: 1 }} />
        {execution.profile && (
          <span style={{ color: "#78716c", fontSize: 10 }}>{execution.profile}</span>
        )}
        {execution.id && (
          <span style={{ color: "#44403c", fontSize: 10 }}>{execution.id}</span>
        )}
      </div>

      {/* ── Output distribution ── */}
      {Array.isArray(output) && (
        <div style={{ padding: "6px 16px", borderBottom: "1px solid #292524", display: "flex", alignItems: "center", gap: 8 }}>
          <div style={{ flex: 1, display: "flex", height: 4, borderRadius: 2, overflow: "hidden", gap: 1 }}>
            {output.map((s, i) => (
              <div key={i} style={{ flex: s, background: scoreColor(s), minWidth: s > 0 ? 2 : 0 }} />
            ))}
          </div>
        </div>
      )}

      {/* ── Reasoning ── */}
      {reasoning && (
        <div style={{ padding: "6px 16px", borderBottom: "1px solid #292524", color: "#78716c", fontSize: 10 }}>
          <span style={{ color: "#d97706", fontWeight: 700, marginRight: 6 }}>R</span>
          {reasoning.length > 140 ? reasoning.slice(0, 137) + "…" : reasoning}
        </div>
      )}

      {/* ── Tasks ── */}
      {tasks.map((task, ti) => {
        const tState = taskState(task);
        const scores: number[] = task.scores ?? [];
        const pathKey = (task.task_path ?? [ti]).join(",");
        const labels = responseLabels?.[pathKey] ?? null;
        const prompt = definition?.tasks?.[ti]?.messages;
        const promptText = prompt
          ? (prompt.find((m: any) => m.role === "system") as any)?.content ??
            (prompt.find((m: any) => m.role === "user") as any)?.content ?? null
          : null;
        const profileTask = profile?.tasks?.[ti];
        const taskWeight = profile?.profile?.[ti] ?? null;
        const votes = task.votes ?? [];
        const maxWeight = votes.length > 0 ? Math.max(...votes.map(v => v.weight)) : 1;

        return (
          <div
            key={ti}
            style={{
              borderBottom: ti < tasks.length - 1 ? "1px solid #292524" : "none",
              borderLeft: taskWeight !== null ? `2px solid #292524` : "none",
            }}
          >
            {/* Task header row */}
            <div style={{ padding: "8px 14px 4px", display: "flex", alignItems: "baseline", gap: 8 }}>
              <span style={{ color: stateColor(tState), fontSize: 7 }}>●</span>
              <span style={{ fontWeight: 600, fontSize: 12 }}>Task {ti}</span>
              {taskWeight !== null && (
                <span style={{ color: "#78716c", fontSize: 10 }}>w={taskWeight.toFixed(1)}</span>
              )}
              {promptText && (
                <span style={{ color: "#57534e", fontSize: 10, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {typeof promptText === "string" ? (promptText.length > 90 ? promptText.slice(0, 87) + "…" : promptText) : "[expr]"}
                </span>
              )}
              <span style={{ color: "#78716c", fontSize: 10 }}>{votes.length} LLMs</span>
            </div>

            {/* Aggregated score bars */}
            {scores.length > 0 && (
              <div style={{ padding: "2px 14px 6px" }}>
                {scores.map((score, si) => {
                  const label = labels?.[si] ?? `#${si + 1}`;
                  return (
                    <div key={si} style={{ display: "flex", alignItems: "center", height: 20 }}>
                      <span style={{ color: "#78716c", width: 76, textAlign: "right", fontSize: 10, paddingRight: 8, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {label}
                      </span>
                      <div style={{ flex: 1, height: 6, background: "#292524", borderRadius: 3, overflow: "hidden" }}>
                        <div style={{ width: `${score * 100}%`, height: "100%", background: scoreColor(score), borderRadius: 3 }} />
                      </div>
                      <span style={{ color: scoreColor(score), width: 48, textAlign: "right", fontSize: 10, fontWeight: 600, paddingLeft: 6 }}>
                        {(score * 100).toFixed(1)}%
                      </span>
                    </div>
                  );
                })}
              </div>
            )}

            {/* Streaming state — no scores yet */}
            {scores.length === 0 && tState === "streaming" && (
              <div style={{ padding: "4px 14px 8px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={{ color: "#d97706", fontSize: 10 }}>▌</span>
                  <span style={{ color: "#78716c", fontSize: 10, fontStyle: "italic" }}>
                    {(() => {
                      const completions = task.completions;
                      if (!completions || completions.length === 0) return "waiting for votes…";
                      const last = completions[completions.length - 1];
                      const text = last?.choices?.[0]?.delta?.content || last?.choices?.[0]?.message?.content;
                      if (!text) return "running…";
                      return text.length > 100 ? text.slice(0, 97) + "…" : text;
                    })()}
                  </span>
                </div>
              </div>
            )}

            {/* Pending state — no data at all */}
            {scores.length === 0 && tState === "pending" && (
              <div style={{ padding: "4px 14px 8px" }}>
                <span style={{ color: "#292524", fontSize: 10 }}>pending</span>
              </div>
            )}

            {/* LLM rows */}
            {votes.length > 0 && (
              <div style={{ padding: "0 14px 8px" }}>
                {votes.map((vote, vi) => {
                  const name = modelNames?.[vote.model] ? modelNames[vote.model] : vote.model;
                  const shortName = name.includes("/") ? name.split("/").pop()! : name;
                  const llmDef = profileTask?.ensemble?.llms?.[vi];
                  const relWeight = maxWeight > 0 ? vote.weight / maxWeight : 0;

                  return (
                    <div
                      key={vi}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 0,
                        height: 28,
                        borderTop: vi > 0 ? "1px solid #1c1917" : "none",
                      }}
                    >
                      {/* Model name */}
                      <span style={{ width: 120, fontSize: 11, color: "#d6d3d1", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flexShrink: 0 }}>
                        {shortName}
                      </span>

                      {/* Vote distribution — horizontal stacked bar */}
                      <div style={{ width: 100, display: "flex", height: 14, borderRadius: 2, overflow: "hidden", gap: 1, flexShrink: 0 }}>
                        {vote.vote.map((v, i) => (
                          <div key={i} style={{ flex: v, background: scoreColor(v), minWidth: v > 0.01 ? 2 : 0 }} />
                        ))}
                      </div>

                      {/* Favored response */}
                      {labels && (
                        <span style={{ color: "#78716c", fontSize: 9, width: 70, marginLeft: 6, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flexShrink: 0 }}>
                          → {labels[vote.vote.indexOf(Math.max(...vote.vote))] ?? `#${vote.vote.indexOf(Math.max(...vote.vote)) + 1}`}
                        </span>
                      )}

                      {/* Weight bar */}
                      <div style={{ width: 50, marginLeft: 10, display: "flex", alignItems: "center", gap: 4 }}>
                        <div style={{ width: 24, height: 4, background: "#292524", borderRadius: 2, overflow: "hidden" }}>
                          <div style={{ width: `${relWeight * 100}%`, height: "100%", background: "#78716c", borderRadius: 2 }} />
                        </div>
                        <span style={{ color: "#78716c", fontSize: 9 }}>{vote.weight.toFixed(2)}</span>
                      </div>

                      {/* Badges */}
                      <div style={{ display: "flex", gap: 4, marginLeft: 8 }}>
                        {vote.from_cache && (
                          <span style={{ color: "#141210", background: "#78716c", borderRadius: 2, padding: "0 3px", fontSize: 8, lineHeight: "14px" }}>C</span>
                        )}
                        {vote.from_rng && (
                          <span style={{ color: "#141210", background: "#d97706", borderRadius: 2, padding: "0 3px", fontSize: 8, lineHeight: "14px" }}>R</span>
                        )}
                      </div>

                      {/* Output mode + logprobs */}
                      <span style={{ color: "#57534e", fontSize: 9, marginLeft: "auto", whiteSpace: "nowrap" }}>
                        {llmDef?.output_mode?.replace(/_/g, " ") ?? ""}
                        {llmDef?.top_logprobs ? ` · top${llmDef.top_logprobs}` : ""}
                      </span>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// PROTOTYPE 2: Sparkline Table / Heatmap
//
// Agents as rows, tasks as columns. Each cell shows vote distribution.
// Column headers show aggregated scores. Cross-model comparison is instant.
// ---------------------------------------------------------------------------

export function SparklineTable({ execution, profile, modelNames, responseLabels }: PrototypeProps) {
  // Collect unique model IDs across all tasks
  const allModels = useMemo(() => {
    const models = new Map<string, string>();
    for (const task of (execution.tasks ?? []) as ProtoTask[]) {
      for (const vote of task.votes ?? []) {
        if (!models.has(vote.model)) {
          const name = modelNames?.[vote.model] ?? vote.model;
          models.set(vote.model, name.includes("/") ? name.split("/").pop()! : name);
        }
      }
    }
    return models;
  }, [execution, modelNames]);

  const tasks = (execution.tasks ?? []) as ProtoTask[];

  return (
    <div
      style={{
        ...mono,
        fontSize: 11,
        color: "#d6d3d1",
        background: "#141210",
        border: "1px solid #292524",
        borderRadius: 4,
        overflow: "auto",
      }}
    >
      <table style={{ width: "100%", borderCollapse: "collapse" }}>
        <thead>
          <tr style={{ borderBottom: "1px solid #292524" }}>
            <th style={{ padding: "10px 12px", textAlign: "left", color: "#78716c", fontWeight: 400, position: "sticky", left: 0, background: "#141210", zIndex: 1, minWidth: 120 }}>
              model
            </th>
            {tasks.map((task, ti) => {
              const pathKey = (task.task_path ?? [ti]).join(",");
              const labels = responseLabels?.[pathKey] ?? null;
              const scores: number[] = task.scores ?? [];
              const maxIdx = scores.length > 0 ? scores.indexOf(Math.max(...scores)) : -1;

              return (
                <th key={ti} style={{ padding: "10px 12px", textAlign: "left", fontWeight: 400, minWidth: 180 }}>
                  <div style={{ color: "#d6d3d1", fontWeight: 600, marginBottom: 4 }}>Task {ti}</div>
                  {/* Aggregated scores */}
                  {scores.length > 0 && (
                    <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                      {scores.map((s, si) => (
                        <div key={si} style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 10 }}>
                          <div style={{ width: 60, height: 5, background: "#292524", borderRadius: 2, overflow: "hidden" }}>
                            <div style={{ width: `${s * 100}%`, height: "100%", background: scoreColor(s), borderRadius: 2 }} />
                          </div>
                          <span style={{ color: si === maxIdx ? scoreColor(s) : "#78716c" }}>
                            {labels?.[si] ?? `#${si + 1}`} {(s * 100).toFixed(0)}%
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </th>
              );
            })}
            <th style={{ padding: "10px 12px", textAlign: "left", color: "#78716c", fontWeight: 400, minWidth: 60 }}>
              weight
            </th>
          </tr>
        </thead>
        <tbody>
          {Array.from(allModels.entries()).map(([modelId, modelName]) => {
            // Find this model's weight from first task where it appears
            let weight: number | null = null;
            for (const task of tasks) {
              const vote = task.votes?.find((v) => v.model === modelId);
              if (vote) { weight = vote.weight; break; }
            }

            return (
              <tr key={modelId} style={{ borderBottom: "1px solid #1c1917" }}>
                {/* Model name */}
                <td style={{ padding: "8px 12px", position: "sticky", left: 0, background: "#141210", zIndex: 1 }}>
                  <span style={{ color: "#d6d3d1" }}>{modelName}</span>
                </td>
                {/* Per-task vote cells */}
                {tasks.map((task, ti) => {
                  const vote = task.votes?.find((v) => v.model === modelId);
                  if (!vote) {
                    return (
                      <td key={ti} style={{ padding: "8px 12px", color: "#292524" }}>—</td>
                    );
                  }
                  const pathKey = (task.task_path ?? [ti]).join(",");
                  const labels = responseLabels?.[pathKey] ?? null;

                  return (
                    <td key={ti} style={{ padding: "8px 12px" }}>
                      {/* Vote distribution as horizontal stacked bar */}
                      <div style={{ display: "flex", height: 14, borderRadius: 2, overflow: "hidden", gap: 1, marginBottom: 3 }}>
                        {vote.vote.map((v, i) => (
                          <div
                            key={i}
                            style={{
                              flex: v,
                              background: scoreColor(v),
                              minWidth: v > 0.01 ? 2 : 0,
                            }}
                          />
                        ))}
                      </div>
                      {/* Labels under bar */}
                      <div style={{ display: "flex", gap: 6, fontSize: 9, color: "#78716c" }}>
                        {vote.vote.map((v, i) => (
                          <span key={i}>
                            {labels?.[i] ? labels[i].slice(0, 6) : `#${i + 1}`}: {(v * 100).toFixed(0)}%
                          </span>
                        ))}
                      </div>
                      {/* Badges */}
                      <div style={{ marginTop: 2 }}>
                        {vote.from_cache && (
                          <span style={{ color: "#78716c", fontSize: 9 }}>CACHE </span>
                        )}
                        {vote.from_rng && (
                          <span style={{ color: "#d97706", fontSize: 9 }}>RNG </span>
                        )}
                      </div>
                    </td>
                  );
                })}
                {/* Weight column */}
                <td style={{ padding: "8px 12px", color: "#78716c" }}>
                  {weight !== null ? weight.toFixed(2) : "—"}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

// ---------------------------------------------------------------------------
// PROTOTYPE 3: Icicle / Flame Chart
//
// Horizontal bands showing execution hierarchy. Width encodes weight.
// Click to zoom into a layer. Each band shows inline score data.
// ---------------------------------------------------------------------------

export function IcicleView({ execution, definition, profile, modelNames, responseLabels }: PrototypeProps) {
  const output = execution.output as number[] | number | null;
  const tasks = (execution.tasks ?? []) as ProtoTask[];

  return (
    <div
      style={{
        ...mono,
        fontSize: 11,
        color: "#d6d3d1",
        background: "#141210",
        border: "1px solid #292524",
        borderRadius: 4,
        overflow: "hidden",
      }}
    >
      {/* Root band */}
      <div
        style={{
          padding: "10px 16px",
          background: "#1c1917",
          borderBottom: "1px solid #292524",
          display: "flex",
          alignItems: "center",
          gap: 12,
        }}
      >
        <span style={{ fontWeight: 700, fontSize: 13 }}>
          {execution.function?.split("/").pop() || "Function"}
        </span>
        {output !== null && (
          <span style={{ color: "#f59e0b", fontWeight: 700 }}>
            {Array.isArray(output)
              ? `${(Math.max(...output) * 100).toFixed(1)}%`
              : `${((output as number) * 100).toFixed(1)}%`}
          </span>
        )}
        {/* Output stacked bar */}
        {Array.isArray(output) && (
          <div style={{ flex: 1, display: "flex", height: 8, borderRadius: 4, overflow: "hidden", gap: 1 }}>
            {output.map((s, i) => (
              <div key={i} style={{ flex: s, background: scoreColor(s), minWidth: s > 0 ? 2 : 0 }} />
            ))}
          </div>
        )}
      </div>

      {/* Task bands — width proportional to profile weight */}
      <div style={{ display: "flex", minHeight: 0 }}>
        {tasks.map((task, ti) => {
          const profileWeight = profile?.profile?.[ti] ?? 1;
          const scores: number[] = task.scores ?? [];
          const pathKey = (task.task_path ?? [ti]).join(",");
          const labels = responseLabels?.[pathKey] ?? null;

          return (
            <div
              key={ti}
              style={{
                flex: profileWeight,
                borderRight: ti < tasks.length - 1 ? "1px solid #292524" : "none",
                display: "flex",
                flexDirection: "column",
              }}
            >
              {/* Task header */}
              <div style={{ padding: "8px 12px", borderBottom: "1px solid #1c1917", background: "#1a1816" }}>
                <div style={{ fontWeight: 600, marginBottom: 4 }}>Task {ti}</div>
                {/* Score bars */}
                {scores.map((score, si) => (
                  <div key={si} style={{ display: "flex", alignItems: "center", gap: 6, height: 18 }}>
                    <span style={{ color: "#78716c", width: 60, textAlign: "right", fontSize: 10, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {labels?.[si] ?? `#${si + 1}`}
                    </span>
                    <div style={{ flex: 1, height: 6, background: "#292524", borderRadius: 3, overflow: "hidden" }}>
                      <div style={{ width: `${score * 100}%`, height: "100%", background: scoreColor(score), borderRadius: 3 }} />
                    </div>
                    <span style={{ color: scoreColor(score), fontSize: 10, width: 36, textAlign: "right" }}>
                      {(score * 100).toFixed(0)}%
                    </span>
                  </div>
                ))}
              </div>

              {/* LLM bands — each is a horizontal strip */}
              {task.votes?.map((vote, vi) => {
                const name = modelNames?.[vote.model] ?? vote.model;
                const shortName = name.includes("/") ? name.split("/").pop()! : name;
                const llmWeight = profile?.tasks?.[ti]?.profile?.[vi] ?? vote.weight;
                const maxWeight = Math.max(
                  ...(task.votes?.map((v) => v.weight) ?? [1])
                );
                const relWeight = maxWeight > 0 ? vote.weight / maxWeight : 0.5;

                return (
                  <div
                    key={vi}
                    style={{
                      padding: "5px 12px",
                      borderBottom: "1px solid #1c1917",
                      display: "flex",
                      alignItems: "center",
                      gap: 8,
                      background: `linear-gradient(90deg, rgba(217, 119, 6, ${relWeight * 0.08}) 0%, transparent 100%)`,
                    }}
                  >
                    <span style={{ width: 90, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 10 }}>
                      {shortName}
                    </span>
                    {/* Mini vote dist */}
                    <div style={{ display: "flex", gap: 1, alignItems: "flex-end", height: 12 }}>
                      {vote.vote.map((v, i) => (
                        <div
                          key={i}
                          style={{
                            width: 6,
                            height: Math.max(2, v * 12),
                            background: scoreColor(v),
                            borderRadius: 1,
                          }}
                        />
                      ))}
                    </div>
                    <span style={{ color: "#78716c", fontSize: 9 }}>
                      {vote.weight.toFixed(2)}
                    </span>
                    {vote.from_cache && <span style={{ color: "#78716c", fontSize: 8 }}>C</span>}
                    {vote.from_rng && <span style={{ color: "#d97706", fontSize: 8 }}>R</span>}
                  </div>
                );
              })}
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// PROTOTYPE 4: Decomposition View
//
// Shows how each score was CONSTRUCTED from individual model contributions.
// The core idea: a score is a weighted sum of opinions, not a fact.
// Each model's vote × weight = its contribution to the final score.
// Visualize the assembly, not just the result.
//
// "Nothing is abstracted away. All load-bearing, all visible."
// ---------------------------------------------------------------------------

export function DecompositionView({ execution, definition, profile, modelNames, responseLabels }: PrototypeProps) {
  const output = (execution.output !== undefined && execution.output !== null) ? execution.output as number[] | number : null;
  const state = execState(execution);
  const reasoning = execution.reasoning?.choices?.[0]?.message?.content ?? null;
  const tasks = (execution.tasks ?? []) as ProtoTask[];

  return (
    <div
      style={{
        ...mono,
        fontSize: 11,
        lineHeight: "18px",
        color: "#d6d3d1",
        background: "#0c0a09",
        border: "1px solid #1c1917",
        borderRadius: 4,
        overflow: "hidden",
      }}
    >
      {/* ── Header: identity ── */}
      <div style={{ padding: "12px 20px 10px", borderBottom: "1px solid #1c1917" }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 10 }}>
          <span style={{ color: stateColor(state), fontSize: 6 }}>●</span>
          <span style={{ color: "#a8a29e", fontWeight: 700, fontSize: 14, letterSpacing: "-0.02em" }}>
            {execution.function ?? "Function"}
          </span>
          <span style={{ flex: 1 }} />
          {execution.profile && (
            <span style={{ color: "#57534e", fontSize: 9 }}>{execution.profile}</span>
          )}
          {execution.id && (
            <span style={{ color: "#44403c", fontSize: 9 }}>{execution.id}</span>
          )}
        </div>
      </div>

      {/* ── Result: the judgment ── */}
      {output !== null && (
        <div style={{ padding: "14px 20px", borderBottom: "1px solid #1c1917" }}>
          {Array.isArray(output) ? (
            <>
              {/* Vector output — show each score as a row */}
              {output.map((score, i) => {
                const isWinner = score === Math.max(...output);
                // Get label from first task's response labels (root output spans all tasks)
                return (
                  <div key={i} style={{ display: "flex", alignItems: "center", marginBottom: i < output.length - 1 ? 6 : 0 }}>
                    <span style={{
                      color: isWinner ? "#f5f5f4" : "#57534e",
                      fontSize: isWinner ? 16 : 12,
                      fontWeight: isWinner ? 700 : 400,
                      width: 64,
                      textAlign: "right",
                      paddingRight: 12,
                    }}>
                      {(score * 100).toFixed(1)}%
                    </span>
                    <div style={{ flex: 1, height: isWinner ? 6 : 3, background: "#1c1917", borderRadius: 1 }}>
                      <div style={{
                        width: `${score * 100}%`,
                        height: "100%",
                        background: isWinner ? "#d6d3d1" : "#44403c",
                        borderRadius: 1,
                        transition: "width 0.3s ease",
                      }} />
                    </div>
                  </div>
                );
              })}
            </>
          ) : (
            <div style={{ display: "flex", alignItems: "baseline", gap: 12 }}>
              <span style={{ color: "#f5f5f4", fontSize: 20, fontWeight: 700 }}>
                {((output as number) * 100).toFixed(1)}%
              </span>
              <div style={{ flex: 1, height: 4, background: "#1c1917", borderRadius: 2 }}>
                <div style={{
                  width: `${(output as number) * 100}%`,
                  height: "100%",
                  background: "#d6d3d1",
                  borderRadius: 2,
                }} />
              </div>
            </div>
          )}
        </div>
      )}

      {/* ── Reasoning ── */}
      {reasoning && (
        <div style={{ padding: "8px 20px", borderBottom: "1px solid #1c1917", color: "#57534e", fontSize: 10, fontStyle: "italic" }}>
          {reasoning}
        </div>
      )}

      {/* ── Tasks: decomposed ── */}
      {tasks.map((task, ti) => {
        const tState = taskState(task);
        const scores: number[] = task.scores ?? [];
        const pathKey = (task.task_path ?? [ti]).join(",");
        const labels = responseLabels?.[pathKey] ?? null;
        const votes = task.votes ?? [];
        const taskWeight = profile?.profile?.[ti] ?? null;
        const profileTask = profile?.tasks?.[ti];
        const prompt = definition?.tasks?.[ti]?.messages;
        const promptText = prompt
          ? (prompt.find((m: any) => m.role === "system") as any)?.content ??
            (prompt.find((m: any) => m.role === "user") as any)?.content ?? null
          : null;

        // Compute each model's contribution to the final scores
        // contribution[model][response] = vote[response] × weight / totalWeight
        const totalWeight = votes.reduce((sum, v) => sum + v.weight, 0);

        return (
          <div key={ti} style={{ borderBottom: ti < tasks.length - 1 ? "1px solid #1c1917" : "none" }}>
            {/* Task header — minimal */}
            <div style={{ padding: "12px 20px 6px", display: "flex", alignItems: "baseline", gap: 8 }}>
              <span style={{ color: stateColor(tState), fontSize: 6 }}>●</span>
              <span style={{ color: "#a8a29e", fontWeight: 600, fontSize: 12 }}>Task {ti}</span>
              {taskWeight !== null && (
                <span style={{ color: "#44403c", fontSize: 9 }}>×{taskWeight.toFixed(1)}</span>
              )}
              {promptText && typeof promptText === "string" && (
                <span style={{ color: "#44403c", fontSize: 9, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {promptText.length > 80 ? promptText.slice(0, 77) + "…" : promptText}
                </span>
              )}
            </div>

            {/* Scores with decomposition */}
            {scores.length > 0 && votes.length > 0 && (
              <div style={{ padding: "4px 20px 12px" }}>
                {scores.map((score, si) => {
                  const label = labels?.[si] ?? `Response ${si + 1}`;
                  const isWinner = score === Math.max(...scores);

                  // Each model's contribution to this score
                  const contributions = votes.map((vote, vi) => {
                    const contribution = totalWeight > 0
                      ? (vote.vote[si] * vote.weight) / totalWeight
                      : 0;
                    const name = modelNames?.[vote.model] ?? vote.model;
                    const shortName = name.includes("/") ? name.split("/").pop()! : name;
                    const llmDef = profileTask?.ensemble?.llms?.[vi];
                    return {
                      model: shortName,
                      vote: vote.vote[si],
                      weight: vote.weight,
                      contribution,
                      fromCache: vote.from_cache,
                      fromRng: vote.from_rng,
                      outputMode: llmDef?.output_mode,
                      topLogprobs: llmDef?.top_logprobs,
                    };
                  }).sort((a, b) => b.contribution - a.contribution);

                  return (
                    <div key={si} style={{ marginBottom: si < scores.length - 1 ? 14 : 0 }}>
                      {/* Score header */}
                      <div style={{ display: "flex", alignItems: "baseline", marginBottom: 6 }}>
                        <span style={{
                          color: isWinner ? "#f5f5f4" : "#78716c",
                          fontWeight: isWinner ? 700 : 400,
                          fontSize: isWinner ? 13 : 11,
                          width: 80,
                        }}>
                          {label}
                        </span>
                        <span style={{
                          color: isWinner ? "#f5f5f4" : "#57534e",
                          fontWeight: isWinner ? 700 : 400,
                          fontSize: isWinner ? 13 : 11,
                          width: 56,
                          textAlign: "right",
                        }}>
                          {(score * 100).toFixed(1)}%
                        </span>
                        {/* Aggregated bar */}
                        <div style={{ flex: 1, marginLeft: 10, height: isWinner ? 4 : 2, background: "#1c1917", borderRadius: 1 }}>
                          <div style={{
                            width: `${score * 100}%`,
                            height: "100%",
                            background: isWinner ? "#d6d3d1" : "#44403c",
                            borderRadius: 1,
                          }} />
                        </div>
                      </div>

                      {/* Contribution breakdown — the heart of the view */}
                      {contributions.map((c, ci) => {
                        // Contribution bar width proportional to this model's share of the final score
                        const barWidth = score > 0 ? (c.contribution / score) * 100 : 0;

                        return (
                          <div
                            key={ci}
                            style={{
                              display: "flex",
                              alignItems: "center",
                              height: 20,
                              paddingLeft: 12,
                            }}
                          >
                            {/* Model name */}
                            <span style={{ width: 100, color: "#57534e", fontSize: 10, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                              {c.model}
                            </span>

                            {/* Vote value */}
                            <span style={{ width: 36, color: "#57534e", fontSize: 9, textAlign: "right", paddingRight: 6 }}>
                              {(c.vote * 100).toFixed(0)}%
                            </span>

                            {/* × weight */}
                            <span style={{ width: 10, color: "#292524", fontSize: 9, textAlign: "center" }}>×</span>
                            <span style={{ width: 28, color: "#57534e", fontSize: 9, paddingLeft: 2 }}>
                              {c.weight.toFixed(2)}
                            </span>

                            {/* → contribution bar */}
                            <span style={{ width: 14, color: "#292524", fontSize: 9, textAlign: "center" }}>→</span>
                            <div style={{ flex: 1, height: 10, background: "#1c1917", borderRadius: 1, position: "relative" }}>
                              <div style={{
                                width: `${barWidth}%`,
                                height: "100%",
                                background: isWinner ? "#78716c" : "#292524",
                                borderRadius: 1,
                                minWidth: c.contribution > 0 ? 2 : 0,
                              }} />
                            </div>

                            {/* Contribution % */}
                            <span style={{ width: 40, color: "#44403c", fontSize: 9, textAlign: "right", paddingLeft: 6 }}>
                              {(c.contribution * 100).toFixed(1)}%
                            </span>

                            {/* Flags */}
                            <div style={{ width: 30, display: "flex", gap: 3, paddingLeft: 6 }}>
                              {c.fromCache && <span style={{ color: "#292524", fontSize: 8 }}>C</span>}
                              {c.fromRng && <span style={{ color: "#44403c", fontSize: 8 }}>R</span>}
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  );
                })}
              </div>
            )}

            {/* Streaming fallback */}
            {scores.length === 0 && tState === "streaming" && (
              <div style={{ padding: "4px 20px 12px" }}>
                <span style={{ color: "#57534e", fontSize: 10, fontStyle: "italic" }}>
                  ▌ {(() => {
                    const completions = task.completions;
                    if (!completions || completions.length === 0) return "waiting…";
                    const last = completions[completions.length - 1];
                    const text = last?.choices?.[0]?.delta?.content || last?.choices?.[0]?.message?.content;
                    if (!text) return "running…";
                    return text.length > 100 ? text.slice(0, 97) + "…" : text;
                  })()}
                </span>
              </div>
            )}

            {scores.length === 0 && tState === "pending" && (
              <div style={{ padding: "4px 20px 12px" }}>
                <span style={{ color: "#292524", fontSize: 10 }}>pending</span>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// PROTOTYPE 5: Vote Matrix — The Math Made Visible
//
// This IS the matrix multiplication. Models as rows, responses as columns.
// Each cell: vote probability as heat-colored background + numeric value.
// Weight column on the left (weight is the LENS — prominent, not hidden).
// Score row at the bottom (the product of the multiplication).
//
// From research: "leading-dot format (.72 not 0.72)", heat-tinted backgrounds,
// 2-3 char codes, fits 272px height for 2 tasks.
// Structural analog: options chain tables (dense, numeric, heat-mapped).
// ---------------------------------------------------------------------------

export function VoteMatrix({ execution, definition, profile, modelNames, responseLabels }: PrototypeProps) {
  const output = (execution.output !== undefined && execution.output !== null) ? execution.output as number[] | number : null;
  const state = execState(execution);
  const reasoning = execution.reasoning?.choices?.[0]?.message?.content ?? null;
  const tasks = (execution.tasks ?? []) as ProtoTask[];

  return (
    <div
      style={{
        ...mono,
        fontSize: 11,
        lineHeight: "16px",
        color: "#d6d3d1",
        background: "#0c0a09",
        border: "1px solid #1c1917",
        borderRadius: 4,
        overflow: "hidden",
      }}
    >
      {/* ── Header ── */}
      <div style={{ padding: "10px 16px 8px", borderBottom: "1px solid #1c1917", display: "flex", alignItems: "center", gap: 10 }}>
        <span style={{ color: stateColor(state), fontSize: 6 }}>●</span>
        <span style={{ color: "#a8a29e", fontWeight: 700, fontSize: 13, letterSpacing: "-0.02em" }}>
          {execution.function ?? "Function"}
        </span>
        {output !== null && (
          <span style={{ color: "#f59e0b", fontWeight: 700, fontSize: 13 }}>
            {Array.isArray(output)
              ? `#${output.indexOf(Math.max(...output)) + 1} · ${(Math.max(...output) * 100).toFixed(1)}%`
              : `${((output as number) * 100).toFixed(1)}%`}
          </span>
        )}
        {state === "streaming" && output === null && (
          <span style={{ color: "#d97706", fontSize: 11 }}>running…</span>
        )}
        <span style={{ flex: 1 }} />
        {execution.id && <span style={{ color: "#44403c", fontSize: 9 }}>{execution.id}</span>}
      </div>

      {/* ── Reasoning ── */}
      {reasoning && (
        <div style={{ padding: "6px 16px", borderBottom: "1px solid #1c1917", color: "#57534e", fontSize: 9, fontStyle: "italic" }}>
          {reasoning.length > 140 ? reasoning.slice(0, 137) + "…" : reasoning}
        </div>
      )}

      {/* ── Per-task matrix ── */}
      {tasks.map((task, ti) => {
        const tState = taskState(task);
        const scores: number[] = task.scores ?? [];
        const pathKey = (task.task_path ?? [ti]).join(",");
        const labels = responseLabels?.[pathKey] ?? null;
        const votes = task.votes ?? [];
        const taskWeight = profile?.profile?.[ti] ?? null;
        const totalWeight = votes.reduce((sum, v) => sum + v.weight, 0);
        const numResponses = scores.length || (votes[0]?.vote.length ?? 0);

        return (
          <div key={ti} style={{ borderBottom: ti < tasks.length - 1 ? "1px solid #1c1917" : "none" }}>
            {/* Task label */}
            <div style={{ padding: "8px 16px 4px", display: "flex", alignItems: "baseline", gap: 8 }}>
              <span style={{ color: stateColor(tState), fontSize: 6 }}>●</span>
              <span style={{ color: "#a8a29e", fontWeight: 600, fontSize: 11 }}>Task {ti}</span>
              {taskWeight !== null && <span style={{ color: "#44403c", fontSize: 9 }}>×{taskWeight.toFixed(1)}</span>}
            </div>

            {votes.length > 0 && numResponses > 0 ? (
              <div style={{ padding: "0 16px 10px", overflowX: "auto" }}>
                <table style={{ borderCollapse: "collapse", width: "100%" }}>
                  {/* Column headers: response labels */}
                  <thead>
                    <tr>
                      {/* Weight column header */}
                      <th style={{ padding: "4px 8px 4px 0", textAlign: "right", color: "#44403c", fontSize: 9, fontWeight: 400, width: 36 }}>
                        w
                      </th>
                      {/* Model name column */}
                      <th style={{ padding: "4px 8px", textAlign: "left", color: "#44403c", fontSize: 9, fontWeight: 400, width: 100 }}>
                        model
                      </th>
                      {/* Response columns */}
                      {Array.from({ length: numResponses }).map((_, ri) => (
                        <th key={ri} style={{
                          padding: "4px 6px",
                          textAlign: "center",
                          color: "#78716c",
                          fontSize: 9,
                          fontWeight: 500,
                          minWidth: 52,
                          borderBottom: "1px solid #1c1917",
                        }}>
                          {labels?.[ri]
                            ? labels[ri].length > 8 ? labels[ri].slice(0, 7) + "…" : labels[ri]
                            : `R${ri + 1}`}
                        </th>
                      ))}
                      {/* Flags column */}
                      <th style={{ padding: "4px 4px", textAlign: "center", color: "#44403c", fontSize: 8, fontWeight: 400, width: 24 }} />
                    </tr>
                  </thead>
                  <tbody>
                    {votes.map((vote, vi) => {
                      const name = modelNames?.[vote.model] ?? vote.model;
                      const shortName = name.includes("/") ? name.split("/").pop()! : name;
                      const maxVoteVal = Math.max(...vote.vote);
                      const maxWeight = Math.max(...votes.map(v => v.weight));
                      const relWeight = maxWeight > 0 ? vote.weight / maxWeight : 0;

                      return (
                        <tr key={vi}>
                          {/* Weight — the LENS. Visual bar + number. */}
                          <td style={{ padding: "3px 8px 3px 0", textAlign: "right", verticalAlign: "middle" }}>
                            <div style={{ display: "flex", alignItems: "center", justifyContent: "flex-end", gap: 4 }}>
                              <div style={{
                                width: 16, height: 12,
                                background: "#1c1917",
                                borderRadius: 1,
                                overflow: "hidden",
                                display: "flex",
                                alignItems: "flex-end",
                              }}>
                                <div style={{
                                  width: "100%",
                                  height: `${relWeight * 100}%`,
                                  background: `rgba(245, 158, 11, ${0.3 + relWeight * 0.5})`,
                                  borderRadius: 1,
                                }} />
                              </div>
                              <span style={{ color: "#78716c", fontSize: 9, fontVariantNumeric: "tabular-nums" }}>
                                {vote.weight.toFixed(2)}
                              </span>
                            </div>
                          </td>
                          {/* Model name */}
                          <td style={{
                            padding: "3px 8px",
                            color: "#a8a29e",
                            fontSize: 10,
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                            maxWidth: 100,
                          }}>
                            {shortName}
                          </td>
                          {/* Vote cells — heat-mapped */}
                          {vote.vote.map((v, ri) => {
                            const isMax = v === maxVoteVal && v > 0;
                            // Heat: opacity scales with vote probability
                            const heat = v > 0.01 ? `rgba(245, 158, 11, ${v * 0.35})` : "transparent";

                            return (
                              <td
                                key={ri}
                                style={{
                                  padding: "3px 6px",
                                  textAlign: "center",
                                  background: heat,
                                  color: isMax ? "#f5f5f4" : v > 0.2 ? "#a8a29e" : "#57534e",
                                  fontSize: 10,
                                  fontWeight: isMax ? 600 : 400,
                                  fontVariantNumeric: "tabular-nums",
                                  borderLeft: "1px solid #1c1917",
                                }}
                              >
                                .{(v * 100).toFixed(0).padStart(2, "0")}
                              </td>
                            );
                          })}
                          {/* Flags */}
                          <td style={{ padding: "3px 4px", textAlign: "center", fontSize: 8 }}>
                            {vote.from_cache && <span style={{ color: "#57534e" }}>C</span>}
                            {vote.from_rng && <span style={{ color: "#d97706" }}>R</span>}
                          </td>
                        </tr>
                      );
                    })}
                    {/* ── Score row (Σ) — the product ── */}
                    {scores.length > 0 && (
                      <tr style={{ borderTop: "2px solid #292524" }}>
                        <td style={{ padding: "5px 8px 5px 0" }} />
                        <td style={{ padding: "5px 8px", color: "#78716c", fontSize: 9, fontWeight: 600 }}>
                          Σ
                        </td>
                        {scores.map((score, ri) => {
                          const isWinner = score === Math.max(...scores);
                          return (
                            <td
                              key={ri}
                              style={{
                                padding: "5px 6px",
                                textAlign: "center",
                                color: isWinner ? "#f5f5f4" : "#78716c",
                                fontSize: isWinner ? 12 : 10,
                                fontWeight: isWinner ? 700 : 500,
                                fontVariantNumeric: "tabular-nums",
                                borderLeft: "1px solid #1c1917",
                                background: isWinner ? "rgba(245, 158, 11, 0.12)" : "transparent",
                              }}
                            >
                              .{(score * 100).toFixed(0).padStart(2, "0")}
                            </td>
                          );
                        })}
                        <td />
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            ) : tState === "streaming" ? (
              <div style={{ padding: "4px 16px 10px" }}>
                <span style={{ color: "#57534e", fontSize: 10, fontStyle: "italic" }}>
                  ▌ {(() => {
                    const completions = task.completions;
                    if (!completions || completions.length === 0) return "waiting…";
                    const last = completions[completions.length - 1];
                    const text = last?.choices?.[0]?.delta?.content || last?.choices?.[0]?.message?.content;
                    if (!text) return "running…";
                    return text.length > 100 ? text.slice(0, 97) + "…" : text;
                  })()}
                </span>
              </div>
            ) : tState === "pending" ? (
              <div style={{ padding: "4px 16px 10px" }}>
                <span style={{ color: "#292524", fontSize: 10 }}>pending</span>
              </div>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// PROTOTYPE 5B: Vote Matrix V2 — Refined
//
// Fixes from V1: full response labels (no truncation), contribution bars
// inside cells showing vote×weight share, tighter vertical rhythm,
// weight fader more prominent.
// ---------------------------------------------------------------------------

export function VoteMatrixV2({ execution, definition, profile, modelNames, responseLabels }: PrototypeProps) {
  const output = (execution.output !== undefined && execution.output !== null) ? execution.output as number[] | number : null;
  const state = execState(execution);
  const reasoning = execution.reasoning?.choices?.[0]?.message?.content ?? null;
  const tasks = (execution.tasks ?? []) as ProtoTask[];

  return (
    <div
      style={{
        ...mono,
        fontSize: 11,
        lineHeight: "16px",
        color: "#d6d3d1",
        background: "#0c0a09",
        border: "1px solid #1c1917",
        borderRadius: 4,
        overflow: "hidden",
      }}
    >
      {/* ── Header ── */}
      <div style={{ padding: "10px 16px 8px", borderBottom: "1px solid #1c1917", display: "flex", alignItems: "center", gap: 10 }}>
        <span style={{ color: stateColor(state), fontSize: 6 }}>●</span>
        <span style={{ color: "#a8a29e", fontWeight: 700, fontSize: 13, letterSpacing: "-0.02em" }}>
          {execution.function ?? "Function"}
        </span>
        {output !== null && (
          <span style={{ color: "#f59e0b", fontWeight: 700, fontSize: 13 }}>
            {Array.isArray(output)
              ? `#${output.indexOf(Math.max(...output)) + 1} · ${(Math.max(...output) * 100).toFixed(1)}%`
              : `${((output as number) * 100).toFixed(1)}%`}
          </span>
        )}
        {state === "streaming" && output === null && (
          <span style={{ color: "#d97706", fontSize: 11 }}>running…</span>
        )}
        <span style={{ flex: 1 }} />
        {execution.profile && <span style={{ color: "#57534e", fontSize: 9, marginRight: 8 }}>{execution.profile}</span>}
        {execution.id && <span style={{ color: "#44403c", fontSize: 9 }}>{execution.id}</span>}
      </div>

      {/* ── Reasoning ── */}
      {reasoning && (
        <div style={{ padding: "6px 16px", borderBottom: "1px solid #1c1917", color: "#57534e", fontSize: 9, fontStyle: "italic" }}>
          {reasoning.length > 140 ? reasoning.slice(0, 137) + "…" : reasoning}
        </div>
      )}

      {/* ── Per-task matrix ── */}
      {tasks.map((task, ti) => {
        const tState = taskState(task);
        const scores: number[] = task.scores ?? [];
        const pathKey = (task.task_path ?? [ti]).join(",");
        const labels = responseLabels?.[pathKey] ?? null;
        const votes = task.votes ?? [];
        const taskWeight = profile?.profile?.[ti] ?? null;
        const totalWeight = votes.reduce((sum, v) => sum + v.weight, 0);
        const numResponses = scores.length || (votes[0]?.vote.length ?? 0);
        const maxWeight = votes.length > 0 ? Math.max(...votes.map(v => v.weight)) : 1;
        const profileTask = profile?.tasks?.[ti];

        return (
          <div key={ti} style={{ borderBottom: ti < tasks.length - 1 ? "1px solid #1c1917" : "none" }}>
            {/* Task label */}
            <div style={{ padding: "8px 16px 4px", display: "flex", alignItems: "baseline", gap: 8 }}>
              <span style={{ color: stateColor(tState), fontSize: 6 }}>●</span>
              <span style={{ color: "#a8a29e", fontWeight: 600, fontSize: 11 }}>Task {ti}</span>
              {taskWeight !== null && <span style={{ color: "#44403c", fontSize: 9 }}>×{taskWeight.toFixed(1)}</span>}
              <span style={{ color: "#44403c", fontSize: 9 }}>{votes.length} agents</span>
              {/* Prompt preview */}
              {(() => {
                const prompt = definition?.tasks?.[ti]?.messages;
                const text = prompt
                  ? (prompt.find((m: any) => m.role === "system") as any)?.content ?? null
                  : null;
                return text && typeof text === "string" ? (
                  <span style={{ color: "#292524", fontSize: 9, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {text.length > 70 ? text.slice(0, 67) + "…" : text}
                  </span>
                ) : null;
              })()}
            </div>

            {votes.length > 0 && numResponses > 0 ? (
              <div style={{ padding: "0 16px 10px", overflowX: "auto" }}>
                <table style={{ borderCollapse: "collapse", width: "100%" }}>
                  <thead>
                    <tr>
                      {/* Weight column */}
                      <th style={{ padding: "4px 6px 6px 0", textAlign: "right", color: "#44403c", fontSize: 8, fontWeight: 400, width: 52, verticalAlign: "bottom" }}>
                        weight
                      </th>
                      {/* Model column */}
                      <th style={{ padding: "4px 8px 6px", textAlign: "left", color: "#44403c", fontSize: 8, fontWeight: 400, width: 140, verticalAlign: "bottom" }}>
                        agent
                      </th>
                      {/* Response columns — FULL labels, no truncation */}
                      {Array.from({ length: numResponses }).map((_, ri) => {
                        const label = labels?.[ri] ?? `Response ${ri + 1}`;
                        const isWinnerCol = scores.length > 0 && scores[ri] === Math.max(...scores);
                        return (
                          <th key={ri} style={{
                            padding: "4px 8px 6px",
                            textAlign: "center",
                            color: isWinnerCol ? "#a8a29e" : "#57534e",
                            fontSize: 9,
                            fontWeight: isWinnerCol ? 600 : 400,
                            minWidth: 80,
                            borderBottom: isWinnerCol ? "2px solid #f59e0b" : "1px solid #1c1917",
                            verticalAlign: "bottom",
                          }}>
                            {label}
                          </th>
                        );
                      })}
                      {/* Flags */}
                      <th style={{ width: 24 }} />
                    </tr>
                  </thead>
                  <tbody>
                    {votes.map((vote, vi) => {
                      const name = modelNames?.[vote.model] ?? vote.model;
                      const shortName = name.includes("/") ? name.split("/").pop()! : name;
                      const maxVoteVal = Math.max(...vote.vote);
                      const relWeight = maxWeight > 0 ? vote.weight / maxWeight : 0;
                      const llmDef = profileTask?.ensemble?.llms?.[vi];

                      return (
                        <tr key={vi} style={{
                          borderTop: vi > 0 ? "1px solid #0f0d0c" : "none",
                          // Weight as ambient glow: higher weight = subtle warm tint on entire row
                          background: relWeight > 0.7 ? `rgba(245, 158, 11, ${relWeight * 0.04})` : "transparent",
                        }}>
                          {/* Weight — vertical fader bar + number. Height encodes weight. */}
                          <td style={{ padding: "2px 6px 2px 0", textAlign: "right", verticalAlign: "middle" }}>
                            <div style={{ display: "flex", alignItems: "center", justifyContent: "flex-end", gap: 4 }}>
                              <div style={{
                                width: 4, height: 24,
                                background: "#1c1917",
                                borderRadius: 2,
                                overflow: "hidden",
                                display: "flex",
                                flexDirection: "column",
                                justifyContent: "flex-end",
                              }}>
                                <div style={{
                                  width: "100%",
                                  height: `${relWeight * 100}%`,
                                  background: `rgba(245, 158, 11, ${0.4 + relWeight * 0.5})`,
                                  borderRadius: 2,
                                  transition: "height 0.3s ease",
                                }} />
                              </div>
                              <span style={{ color: "#78716c", fontSize: 9, fontVariantNumeric: "tabular-nums", width: 24, textAlign: "right" }}>
                                {vote.weight.toFixed(2)}
                              </span>
                            </div>
                          </td>
                          {/* Model name + output mode */}
                          <td style={{
                            padding: "4px 8px",
                            verticalAlign: "middle",
                          }}>
                            <span style={{ color: "#a8a29e", fontSize: 10 }}>{shortName}</span>
                            {llmDef?.output_mode && (
                              <span style={{ color: "#292524", fontSize: 8, marginLeft: 4 }}>
                                {llmDef.output_mode === "log_probs" ? "LP" : llmDef.output_mode.slice(0, 3)}
                                {llmDef.top_logprobs ? llmDef.top_logprobs : ""}
                              </span>
                            )}
                          </td>
                          {/* Vote cells — heat + contribution underline */}
                          {vote.vote.map((v: number, ri: number) => {
                            const isMax = v === maxVoteVal && v > 0;
                            const heat = v > 0.01 ? `rgba(245, 158, 11, ${v * 0.3})` : "transparent";
                            // Contribution: how much of the final score this model is responsible for
                            const contribution = totalWeight > 0 ? (v * vote.weight) / totalWeight : 0;
                            const scoreForResponse = scores[ri] ?? 0;
                            const contributionShare = scoreForResponse > 0 ? contribution / scoreForResponse : 0;

                            return (
                              <td
                                key={ri}
                                style={{
                                  padding: "4px 8px 2px",
                                  textAlign: "center",
                                  background: heat,
                                  borderLeft: "1px solid #0f0d0c",
                                  verticalAlign: "middle",
                                }}
                              >
                                {/* Vote value */}
                                <div style={{
                                  color: isMax ? "#f5f5f4" : v > 0.2 ? "#a8a29e" : "#44403c",
                                  fontSize: 10,
                                  fontWeight: isMax ? 600 : 400,
                                  fontVariantNumeric: "tabular-nums",
                                  marginBottom: 2,
                                }}>
                                  .{(v * 100).toFixed(0).padStart(2, "0")}
                                </div>
                                {/* Contribution micro-bar: shows this model's share of the response score */}
                                {scores.length > 0 && (
                                  <div style={{
                                    width: "100%",
                                    height: 3,
                                    background: "#0f0d0c",
                                    borderRadius: 1,
                                  }}>
                                    <div style={{
                                      width: `${Math.min(100, contributionShare * 100)}%`,
                                      height: "100%",
                                      background: isMax ? "rgba(245, 158, 11, 0.6)" : "rgba(120, 113, 108, 0.35)",
                                      borderRadius: 1,
                                    }} />
                                  </div>
                                )}
                              </td>
                            );
                          })}
                          {/* Flags */}
                          <td style={{ padding: "4px 4px", textAlign: "center", fontSize: 8 }}>
                            {vote.from_cache && <span style={{ color: "#57534e" }}>C</span>}
                            {vote.from_rng && <span style={{ color: "#d97706" }}>R</span>}
                          </td>
                        </tr>
                      );
                    })}
                    {/* ── Score row (Σ) ── */}
                    {scores.length > 0 && (
                      <tr style={{ borderTop: "2px solid #292524" }}>
                        <td style={{ padding: "6px 6px 6px 0" }} />
                        <td style={{ padding: "6px 8px", color: "#78716c", fontSize: 9, fontWeight: 600 }}>
                          Σ weighted
                        </td>
                        {scores.map((score: number, ri: number) => {
                          const isWinner = score === Math.max(...scores);
                          return (
                            <td
                              key={ri}
                              style={{
                                padding: "6px 8px",
                                textAlign: "center",
                                borderLeft: "1px solid #0f0d0c",
                                background: isWinner ? "rgba(245, 158, 11, 0.1)" : "transparent",
                              }}
                            >
                              <span style={{
                                color: isWinner ? "#f5f5f4" : "#78716c",
                                fontSize: isWinner ? 13 : 10,
                                fontWeight: isWinner ? 700 : 500,
                                fontVariantNumeric: "tabular-nums",
                              }}>
                                {(score * 100).toFixed(1)}%
                              </span>
                            </td>
                          );
                        })}
                        <td />
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            ) : tState === "streaming" ? (
              <div style={{ padding: "4px 16px 10px" }}>
                <span style={{ color: "#57534e", fontSize: 10, fontStyle: "italic" }}>
                  ▌ {(() => {
                    const completions = task.completions;
                    if (!completions || completions.length === 0) return "waiting…";
                    const last = completions[completions.length - 1];
                    const text = last?.choices?.[0]?.delta?.content || last?.choices?.[0]?.message?.content;
                    if (!text) return "running…";
                    return text.length > 100 ? text.slice(0, 97) + "…" : text;
                  })()}
                </span>
              </div>
            ) : tState === "pending" ? (
              <div style={{ padding: "4px 16px 10px" }}>
                <span style={{ color: "#292524", fontSize: 10 }}>pending</span>
              </div>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// PROTOTYPE 6: Superposition View — The Interference Pattern
//
// Each LLM emits a "wave" — its vote distribution across responses.
// Weight scales the wave's amplitude. Below all individual waves,
// a thick aggregate wave shows the weighted sum.
//
// This IS the math rendered visually, not a metaphor for it.
// When you change weights, waves grow/shrink and the aggregate reshapes.
//
// From research: "signal superposition — literally the computation."
// Responses are positions on a shared x-axis. Amplitude = probability.
// ---------------------------------------------------------------------------

export function SuperpositionView({ execution, definition, profile, modelNames, responseLabels }: PrototypeProps) {
  const output = (execution.output !== undefined && execution.output !== null) ? execution.output as number[] | number : null;
  const state = execState(execution);
  const tasks = (execution.tasks ?? []) as ProtoTask[];

  // Shared dimensions
  const RESPONSE_WIDTH = 72;
  const LABEL_WIDTH = 110;
  const WEIGHT_WIDTH = 48;
  const ROW_HEIGHT = 28;
  const AGGREGATE_HEIGHT = 40;

  return (
    <div
      style={{
        ...mono,
        fontSize: 11,
        lineHeight: "16px",
        color: "#d6d3d1",
        background: "#0c0a09",
        border: "1px solid #1c1917",
        borderRadius: 4,
        overflow: "hidden",
      }}
    >
      {/* ── Header ── */}
      <div style={{ padding: "10px 16px 8px", borderBottom: "1px solid #1c1917", display: "flex", alignItems: "center", gap: 10 }}>
        <span style={{ color: stateColor(state), fontSize: 6 }}>●</span>
        <span style={{ color: "#a8a29e", fontWeight: 700, fontSize: 13 }}>
          {execution.function ?? "Function"}
        </span>
        {output !== null && (
          <span style={{ color: "#f59e0b", fontWeight: 700, fontSize: 13 }}>
            {Array.isArray(output)
              ? `#${output.indexOf(Math.max(...output)) + 1} · ${(Math.max(...output) * 100).toFixed(1)}%`
              : `${((output as number) * 100).toFixed(1)}%`}
          </span>
        )}
        <span style={{ flex: 1 }} />
        {execution.id && <span style={{ color: "#44403c", fontSize: 9 }}>{execution.id}</span>}
      </div>

      {/* ── Per-task superposition ── */}
      {tasks.map((task, ti) => {
        const tState = taskState(task);
        const scores: number[] = task.scores ?? [];
        const pathKey = (task.task_path ?? [ti]).join(",");
        const labels = responseLabels?.[pathKey] ?? null;
        const votes = task.votes ?? [];
        const taskWeight = profile?.profile?.[ti] ?? null;
        const numResponses = scores.length || (votes[0]?.vote.length ?? 0);
        const maxWeight = votes.length > 0 ? Math.max(...votes.map(v => v.weight)) : 1;

        return (
          <div key={ti} style={{ borderBottom: ti < tasks.length - 1 ? "1px solid #1c1917" : "none" }}>
            {/* Task label */}
            <div style={{ padding: "8px 16px 2px", display: "flex", alignItems: "baseline", gap: 8 }}>
              <span style={{ color: stateColor(tState), fontSize: 6 }}>●</span>
              <span style={{ color: "#a8a29e", fontWeight: 600, fontSize: 11 }}>Task {ti}</span>
              {taskWeight !== null && <span style={{ color: "#44403c", fontSize: 9 }}>×{taskWeight.toFixed(1)}</span>}
            </div>

            {votes.length > 0 && numResponses > 0 ? (
              <div style={{ padding: "4px 16px 10px" }}>
                {/* Response column headers — aligned to the wave positions */}
                <div style={{ display: "flex", alignItems: "flex-end", marginBottom: 2 }}>
                  <div style={{ width: WEIGHT_WIDTH, flexShrink: 0 }} />
                  <div style={{ width: LABEL_WIDTH, flexShrink: 0 }} />
                  <div style={{ flex: 1, display: "flex" }}>
                    {Array.from({ length: numResponses }).map((_, ri) => (
                      <div key={ri} style={{
                        width: RESPONSE_WIDTH,
                        textAlign: "center",
                        color: "#57534e",
                        fontSize: 9,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}>
                        {labels?.[ri]
                          ? labels[ri].length > 9 ? labels[ri].slice(0, 8) + "…" : labels[ri]
                          : `R${ri + 1}`}
                      </div>
                    ))}
                  </div>
                </div>

                {/* ── Individual waves: one row per LLM ── */}
                {votes.map((vote, vi) => {
                  const name = modelNames?.[vote.model] ?? vote.model;
                  const shortName = name.includes("/") ? name.split("/").pop()! : name;
                  const relWeight = maxWeight > 0 ? vote.weight / maxWeight : 0;
                  // Amplitude scaling: vote × (weight / maxWeight) for visual proportionality
                  const amplitudeScale = relWeight;

                  return (
                    <div
                      key={vi}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        height: ROW_HEIGHT,
                        borderTop: vi > 0 ? "1px solid #0f0d0c" : "none",
                      }}
                    >
                      {/* Weight — visual fader */}
                      <div style={{ width: WEIGHT_WIDTH, flexShrink: 0, display: "flex", alignItems: "center", gap: 3, justifyContent: "flex-end", paddingRight: 6 }}>
                        <div style={{
                          width: 10, height: 18,
                          background: "#1c1917",
                          borderRadius: 1,
                          overflow: "hidden",
                          display: "flex",
                          alignItems: "flex-end",
                        }}>
                          <div style={{
                            width: "100%",
                            height: `${relWeight * 100}%`,
                            background: `rgba(245, 158, 11, ${0.25 + relWeight * 0.5})`,
                          }} />
                        </div>
                        <span style={{ color: "#57534e", fontSize: 8, fontVariantNumeric: "tabular-nums" }}>
                          {vote.weight.toFixed(1)}
                        </span>
                      </div>

                      {/* Model name */}
                      <div style={{
                        width: LABEL_WIDTH, flexShrink: 0,
                        color: "#a8a29e", fontSize: 10,
                        overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                        paddingRight: 8,
                      }}>
                        {shortName}
                        {vote.from_cache && <span style={{ color: "#44403c", fontSize: 8, marginLeft: 4 }}>C</span>}
                        {vote.from_rng && <span style={{ color: "#d97706", fontSize: 8, marginLeft: 4 }}>R</span>}
                      </div>

                      {/* Wave — bars at response positions, height = vote × amplitude */}
                      <div style={{ flex: 1, display: "flex", alignItems: "flex-end", height: ROW_HEIGHT - 4 }}>
                        {vote.vote.map((v, ri) => {
                          const barHeight = Math.max(1, v * amplitudeScale * (ROW_HEIGHT - 6));
                          const isMax = v === Math.max(...vote.vote) && v > 0;
                          return (
                            <div key={ri} style={{
                              width: RESPONSE_WIDTH,
                              display: "flex",
                              flexDirection: "column",
                              alignItems: "center",
                              justifyContent: "flex-end",
                              height: "100%",
                            }}>
                              <div style={{
                                width: Math.max(4, RESPONSE_WIDTH * 0.6),
                                height: barHeight,
                                background: isMax
                                  ? `rgba(245, 158, 11, ${0.4 + v * 0.5})`
                                  : `rgba(120, 113, 108, ${0.15 + v * 0.3})`,
                                borderRadius: "2px 2px 0 0",
                                transition: "height 0.3s ease",
                              }} />
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  );
                })}

                {/* ── Aggregate wave: the weighted sum ── */}
                {scores.length > 0 && (
                  <div style={{
                    display: "flex",
                    alignItems: "flex-end",
                    height: AGGREGATE_HEIGHT,
                    borderTop: "2px solid #292524",
                    marginTop: 2,
                    paddingTop: 2,
                  }}>
                    <div style={{ width: WEIGHT_WIDTH, flexShrink: 0 }} />
                    <div style={{ width: LABEL_WIDTH, flexShrink: 0, display: "flex", alignItems: "center", height: "100%" }}>
                      <span style={{ color: "#78716c", fontSize: 10, fontWeight: 600 }}>Σ weighted</span>
                    </div>
                    <div style={{ flex: 1, display: "flex", alignItems: "flex-end", height: AGGREGATE_HEIGHT - 4 }}>
                      {scores.map((score, ri) => {
                        const isWinner = score === Math.max(...scores);
                        const barHeight = Math.max(2, score * (AGGREGATE_HEIGHT - 6));
                        return (
                          <div key={ri} style={{
                            width: RESPONSE_WIDTH,
                            display: "flex",
                            flexDirection: "column",
                            alignItems: "center",
                            justifyContent: "flex-end",
                            height: "100%",
                          }}>
                            {/* Score label above bar */}
                            <span style={{
                              color: isWinner ? "#f5f5f4" : "#78716c",
                              fontSize: isWinner ? 11 : 9,
                              fontWeight: isWinner ? 700 : 400,
                              marginBottom: 2,
                              fontVariantNumeric: "tabular-nums",
                            }}>
                              {(score * 100).toFixed(1)}%
                            </span>
                            <div style={{
                              width: Math.max(6, RESPONSE_WIDTH * 0.7),
                              height: barHeight,
                              background: isWinner
                                ? "rgba(245, 158, 11, 0.7)"
                                : "rgba(120, 113, 108, 0.25)",
                              borderRadius: "2px 2px 0 0",
                            }} />
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}
              </div>
            ) : tState === "streaming" ? (
              <div style={{ padding: "4px 16px 10px" }}>
                <span style={{ color: "#57534e", fontSize: 10, fontStyle: "italic" }}>
                  ▌ {(() => {
                    const completions = task.completions;
                    if (!completions || completions.length === 0) return "waiting…";
                    const last = completions[completions.length - 1];
                    const text = last?.choices?.[0]?.delta?.content || last?.choices?.[0]?.message?.content;
                    if (!text) return "running…";
                    return text.length > 100 ? text.slice(0, 97) + "…" : text;
                  })()}
                </span>
              </div>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// PROTOTYPE 7: Contribution Waterfall
//
// For each response, shows how individual model contributions stack up
// to form the final score. Each model is a segment in the bar.
// Width = contribution (vote × weight / totalWeight).
// The segments physically BUILD the score left to right.
//
// This answers the question: "Who is responsible for this judgment?"
// Models that contributed more are literally wider.
// Weight as visible force: higher weight → wider segment → more of the bar.
//
// From research: Marimekko/mosaic plots where width=magnitude.
// "Profiles are perspective, not preference" — swap the profile and
// the segments resize, visibly reshaping the judgment.
// ---------------------------------------------------------------------------

export function ContributionWaterfall({ execution, definition, profile, modelNames, responseLabels }: PrototypeProps) {
  const output = (execution.output !== undefined && execution.output !== null) ? execution.output as number[] | number : null;
  const state = execState(execution);
  const tasks = (execution.tasks ?? []) as ProtoTask[];

  return (
    <div
      style={{
        ...mono,
        fontSize: 11,
        lineHeight: "16px",
        color: "#d6d3d1",
        background: "#0c0a09",
        border: "1px solid #1c1917",
        borderRadius: 4,
        overflow: "hidden",
      }}
    >
      {/* ── Header ── */}
      <div style={{ padding: "10px 16px 8px", borderBottom: "1px solid #1c1917", display: "flex", alignItems: "center", gap: 10 }}>
        <span style={{ color: stateColor(state), fontSize: 6 }}>●</span>
        <span style={{ color: "#a8a29e", fontWeight: 700, fontSize: 13 }}>
          {execution.function ?? "Function"}
        </span>
        {output !== null && (
          <span style={{ color: "#f59e0b", fontWeight: 700, fontSize: 13 }}>
            {Array.isArray(output)
              ? `#${output.indexOf(Math.max(...output)) + 1} · ${(Math.max(...output) * 100).toFixed(1)}%`
              : `${((output as number) * 100).toFixed(1)}%`}
          </span>
        )}
        <span style={{ flex: 1 }} />
        {execution.id && <span style={{ color: "#44403c", fontSize: 9 }}>{execution.id}</span>}
      </div>

      {/* ── Per-task waterfall ── */}
      {tasks.map((task, ti) => {
        const tState = taskState(task);
        const scores: number[] = task.scores ?? [];
        const pathKey = (task.task_path ?? [ti]).join(",");
        const labels = responseLabels?.[pathKey] ?? null;
        const votes = task.votes ?? [];
        const taskWeight = profile?.profile?.[ti] ?? null;
        const totalWeight = votes.reduce((sum, v) => sum + v.weight, 0);

        // Pre-compute contributions per response
        const responseContributions = scores.map((score, si) => {
          return votes.map((vote, vi) => {
            const name = modelNames?.[vote.model] ?? vote.model;
            const shortName = name.includes("/") ? name.split("/").pop()! : name;
            const contribution = totalWeight > 0 ? (vote.vote[si] * vote.weight) / totalWeight : 0;
            return { model: shortName, modelId: vote.model, contribution, vote: vote.vote[si], weight: vote.weight };
          }).sort((a, b) => b.contribution - a.contribution);
        });

        // Unique model colors — assign each model a consistent subtle hue shift
        const modelIds = votes.map(v => v.model);
        const modelHues: Record<string, number> = {};
        modelIds.forEach((id, i) => {
          // Spread across amber spectrum: 25° to 45° hue range
          modelHues[id] = 30 + (i / Math.max(1, modelIds.length - 1)) * 20;
        });

        return (
          <div key={ti} style={{ borderBottom: ti < tasks.length - 1 ? "1px solid #1c1917" : "none" }}>
            {/* Task label */}
            <div style={{ padding: "8px 16px 4px", display: "flex", alignItems: "baseline", gap: 8 }}>
              <span style={{ color: stateColor(tState), fontSize: 6 }}>●</span>
              <span style={{ color: "#a8a29e", fontWeight: 600, fontSize: 11 }}>Task {ti}</span>
              {taskWeight !== null && <span style={{ color: "#44403c", fontSize: 9 }}>×{taskWeight.toFixed(1)}</span>}
            </div>

            {scores.length > 0 && votes.length > 0 ? (
              <div style={{ padding: "2px 16px 12px" }}>
                {scores.map((score, si) => {
                  const label = labels?.[si] ?? `R${si + 1}`;
                  const isWinner = score === Math.max(...scores);
                  const contribs = responseContributions[si];

                  return (
                    <div key={si} style={{ marginBottom: si < scores.length - 1 ? 8 : 0 }}>
                      {/* Response label + score */}
                      <div style={{ display: "flex", alignItems: "baseline", marginBottom: 3, gap: 6 }}>
                        <span style={{
                          color: isWinner ? "#f5f5f4" : "#78716c",
                          fontSize: isWinner ? 12 : 10,
                          fontWeight: isWinner ? 700 : 400,
                          width: 80,
                        }}>
                          {label}
                        </span>
                        <span style={{
                          color: isWinner ? "#f5f5f4" : "#57534e",
                          fontSize: isWinner ? 12 : 10,
                          fontWeight: isWinner ? 700 : 400,
                          fontVariantNumeric: "tabular-nums",
                        }}>
                          {(score * 100).toFixed(1)}%
                        </span>
                      </div>

                      {/* Stacked contribution bar — each segment is one model's contribution */}
                      <div style={{
                        display: "flex",
                        height: isWinner ? 24 : 16,
                        borderRadius: 2,
                        overflow: "hidden",
                        gap: 1,
                        background: "#1c1917",
                      }}>
                        {contribs.map((c, ci) => {
                          // Width proportional to contribution / total score
                          const segWidth = score > 0 ? (c.contribution / score) * 100 : 0;
                          if (segWidth < 0.5) return null;
                          const hue = modelHues[c.modelId] ?? 35;

                          return (
                            <div
                              key={ci}
                              style={{
                                flex: `0 0 ${segWidth}%`,
                                background: isWinner
                                  ? `hsla(${hue}, 80%, 50%, ${0.35 + c.contribution * 2})`
                                  : `hsla(${hue}, 30%, 40%, ${0.15 + c.contribution})`,
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "center",
                                overflow: "hidden",
                                position: "relative",
                              }}
                              title={`${c.model}: vote=${(c.vote * 100).toFixed(0)}% × w=${c.weight.toFixed(2)} → ${(c.contribution * 100).toFixed(1)}%`}
                            >
                              {/* Model initial or name if segment wide enough */}
                              {segWidth > 8 && (
                                <span style={{
                                  color: isWinner ? "#f5f5f4" : "#a8a29e",
                                  fontSize: 8,
                                  whiteSpace: "nowrap",
                                  overflow: "hidden",
                                  textOverflow: "ellipsis",
                                  padding: "0 2px",
                                }}>
                                  {segWidth > 18 ? c.model : c.model.slice(0, 3)}
                                </span>
                              )}
                            </div>
                          );
                        })}
                      </div>

                      {/* Model legend for this response — only for winner */}
                      {isWinner && (
                        <div style={{ display: "flex", flexWrap: "wrap", gap: "2px 10px", marginTop: 3 }}>
                          {contribs.filter(c => c.contribution > 0.01).map((c, ci) => (
                            <span key={ci} style={{ color: "#57534e", fontSize: 8, whiteSpace: "nowrap" }}>
                              {c.model} {(c.contribution * 100).toFixed(1)}%
                            </span>
                          ))}
                        </div>
                      )}
                    </div>
                  );
                })}

                {/* Model weight reference — small, at the bottom */}
                <div style={{ marginTop: 8, paddingTop: 6, borderTop: "1px solid #1c1917", display: "flex", flexWrap: "wrap", gap: "2px 12px" }}>
                  {votes.map((vote, vi) => {
                    const name = modelNames?.[vote.model] ?? vote.model;
                    const shortName = name.includes("/") ? name.split("/").pop()! : name;
                    const maxW = Math.max(...votes.map(v => v.weight));
                    const rel = maxW > 0 ? vote.weight / maxW : 0;
                    return (
                      <div key={vi} style={{ display: "flex", alignItems: "center", gap: 4 }}>
                        <div style={{
                          width: 6, height: 6,
                          borderRadius: 1,
                          background: `hsla(${modelHues[vote.model] ?? 35}, 60%, 50%, ${0.4 + rel * 0.4})`,
                        }} />
                        <span style={{ color: "#57534e", fontSize: 8 }}>
                          {shortName}
                        </span>
                        <span style={{ color: "#44403c", fontSize: 8, fontVariantNumeric: "tabular-nums" }}>
                          {vote.weight.toFixed(2)}
                        </span>
                        {vote.from_cache && <span style={{ color: "#44403c", fontSize: 7 }}>C</span>}
                        {vote.from_rng && <span style={{ color: "#d97706", fontSize: 7 }}>R</span>}
                      </div>
                    );
                  })}
                </div>
              </div>
            ) : tState === "streaming" ? (
              <div style={{ padding: "4px 16px 10px" }}>
                <span style={{ color: "#57534e", fontSize: 10, fontStyle: "italic" }}>
                  ▌ {(() => {
                    const completions = task.completions;
                    if (!completions || completions.length === 0) return "waiting…";
                    const last = completions[completions.length - 1];
                    const text = last?.choices?.[0]?.delta?.content || last?.choices?.[0]?.message?.content;
                    if (!text) return "running…";
                    return text.length > 100 ? text.slice(0, 97) + "…" : text;
                  })()}
                </span>
              </div>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}
