"use client";

import type {
  InputFunctionExecution,
  InputFunctionDefinition,
  InputProfile,
} from "@objectiveai/function-tree/core";

/* eslint-disable @typescript-eslint/no-explicit-any */

interface Task {
  index?: number;
  task_index?: number;
  task_path?: number[];
  votes?: Array<{ model: string; vote: number[]; weight: number; from_cache?: boolean; from_rng?: boolean }>;
  completions?: any[];
  scores?: number[];
  error?: { message?: string } | null;
  tasks?: Task[];
  output?: number | number[];
}

export interface ExecutionResultProps {
  execution: InputFunctionExecution;
  definition?: InputFunctionDefinition | null;
  profile?: InputProfile | null;
  modelNames?: Record<string, string>;
  responseLabels?: Record<string, string[]>;
}

const mono: React.CSSProperties = {
  fontFamily: '"JetBrains Mono", "Fira Code", "SF Mono", "Cascadia Code", monospace',
};

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

export function ExecutionResult({ execution, definition, profile, modelNames, responseLabels }: ExecutionResultProps) {
  const output = (execution.output !== undefined && execution.output !== null) ? execution.output as number[] | number : null;
  const state = execState(execution);
  const reasoning = execution.reasoning?.choices?.[0]?.message?.content ?? null;
  const tasks = (execution.tasks ?? []) as Task[];

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
      {/* Header */}
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

      {/* Reasoning */}
      {reasoning && (
        <div style={{ padding: "6px 16px", borderBottom: "1px solid #1c1917", color: "#57534e", fontSize: 9, fontStyle: "italic" }}>
          {reasoning.length > 140 ? reasoning.slice(0, 137) + "…" : reasoning}
        </div>
      )}

      {/* Per-task matrix */}
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
                      <th style={{ padding: "4px 6px 6px 0", textAlign: "right", color: "#44403c", fontSize: 8, fontWeight: 400, width: 52, verticalAlign: "bottom" }}>
                        weight
                      </th>
                      <th style={{ padding: "4px 8px 6px", textAlign: "left", color: "#44403c", fontSize: 8, fontWeight: 400, width: 140, verticalAlign: "bottom" }}>
                        agent
                      </th>
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
                          background: relWeight > 0.7 ? `rgba(245, 158, 11, ${relWeight * 0.04})` : "transparent",
                        }}>
                          {/* Weight fader */}
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
                          <td style={{ padding: "4px 8px", verticalAlign: "middle" }}>
                            <span style={{ color: "#a8a29e", fontSize: 10 }}>{shortName}</span>
                            {llmDef?.output_mode && (
                              <span style={{ color: "#292524", fontSize: 8, marginLeft: 4 }}>
                                {llmDef.output_mode === "log_probs" ? "LP" : llmDef.output_mode.slice(0, 3)}
                                {llmDef.top_logprobs ? llmDef.top_logprobs : ""}
                              </span>
                            )}
                          </td>
                          {/* Vote cells with heat + contribution bars */}
                          {vote.vote.map((v: number, ri: number) => {
                            const isMax = v === maxVoteVal && v > 0;
                            const heat = v > 0.01 ? `rgba(245, 158, 11, ${v * 0.3})` : "transparent";
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
                                <div style={{
                                  color: isMax ? "#f5f5f4" : v > 0.2 ? "#a8a29e" : "#44403c",
                                  fontSize: 10,
                                  fontWeight: isMax ? 600 : 400,
                                  fontVariantNumeric: "tabular-nums",
                                  marginBottom: 2,
                                }}>
                                  .{(v * 100).toFixed(0).padStart(2, "0")}
                                </div>
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
                    {/* Score row */}
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
