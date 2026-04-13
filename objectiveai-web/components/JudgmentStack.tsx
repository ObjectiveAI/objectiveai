"use client";

import { useState } from "react";
import type { FunctionDefinition, TaskDefinition } from "@/lib/functions/types";
import type { ProfileMeta, ProfileLlm } from "@/lib/profiles/types";
import styles from "./JudgmentStack.module.css";

/* ── Execution types (SDK streaming shape) ── */

interface Vote {
  model: string;
  vote: number[];
  weight: number;
  from_cache?: boolean;
  from_rng?: boolean;
}

interface TaskExecution {
  task_path?: number[];
  votes?: Vote[];
  completions?: Array<Record<string, unknown>>;
  scores?: number[];
  error?: { message?: string } | null;
  tasks?: TaskExecution[];
  output?: number | number[];
}

export interface FunctionExecution {
  id?: string;
  function?: string;
  profile?: string;
  output?: number | number[];
  error?: unknown;
  reasoning?: { choices?: Array<{ message?: { content?: string } }> };
  tasks?: TaskExecution[];
}

/* ── Props ── */

export interface JudgmentStackProps {
  definition?: FunctionDefinition | null;
  execution?: FunctionExecution | null;
  profile?: ProfileMeta | null | undefined;
  resolvedSubFunctions?: Map<string, FunctionDefinition>;
  modelNames?: Record<string, string>;
  depth?: number;
}

/* ── Helpers ── */

function scoreColor(s: number): string {
  if (s >= 0.5) return "var(--copper-hot)";
  if (s >= 0.3) return "var(--copper-mid)";
  if (s >= 0.15) return "var(--copper-warm)";
  return "var(--copper-dim)";
}

function pct(n: number): string {
  return (n * 100).toFixed(1) + "%";
}

function dotPct(n: number): string {
  return "." + (n * 100).toFixed(0).padStart(2, "0");
}

function stateColor(s: string): string {
  if (s === "complete") return "var(--copper-hot)";
  if (s === "streaming") return "var(--copper-mid)";
  if (s === "error") return "#b91c1c";
  return "var(--node-border)";
}

function taskState(exec?: TaskExecution | null): string {
  if (!exec) return "structural";
  if (exec.error) return "error";
  if (exec.scores?.length) return "complete";
  if (exec.completions?.length) return "streaming";
  return "pending";
}

function funcState(exec?: FunctionExecution | null): string {
  if (!exec) return "structural";
  if (exec.error) return "error";
  if (exec.output != null) return "complete";
  if (exec.tasks?.length) return "streaming";
  return "pending";
}

function normalizeType(t: string): string {
  return t.replace(/^alpha\./, "");
}

function shortType(t: string): string {
  const n = normalizeType(t);
  if (n === "vector.completion") return "vc";
  if (n.includes("placeholder")) return "placeholder";
  if (n.includes("vector.function")) return "vector fn";
  if (n.includes("scalar.function")) return "scalar fn";
  return n;
}

function labelFor(r: unknown, i: number): string {
  if (typeof r === "string") return r.length > 24 ? r.slice(0, 22) + "\u2026" : r;
  if (r && typeof r === "object") {
    const o = r as Record<string, unknown>;
    if (typeof o.text === "string") return o.text.length > 24 ? o.text.slice(0, 22) + "\u2026" : o.text;
    if (o.$jmespath || o.$starlark) return "[dynamic]";
  }
  return `#${i + 1}`;
}

function promptPreview(task: TaskDefinition): string | null {
  if (!Array.isArray(task.messages)) return null;
  const msgs = task.messages as Array<Record<string, unknown>>;
  for (const role of ["system", "developer", "user"]) {
    const m = msgs.find((x) => x.role === role);
    if (m && typeof m.content === "string") {
      return m.content.length > 80 ? m.content.slice(0, 77) + "\u2026" : m.content;
    }
  }
  return null;
}

function exprStr(expr: Record<string, unknown> | undefined): string | null {
  if (!expr) return null;
  if (typeof expr.$jmespath === "string") return `jmes: ${expr.$jmespath}`;
  if (typeof expr.$starlark === "string") {
    const s = expr.$starlark as string;
    return s.length > 40 ? `star: ${s.slice(0, 37)}\u2026` : `star: ${s}`;
  }
  return null;
}

function getTaskAgents(
  profile: ProfileMeta | null | undefined,
  ti: number,
): { llms: ProfileLlm[]; weights: number[] } | null {
  if (!profile) return null;
  if (profile.kind === "tasks" && profile.taskConfigs[ti]) return profile.taskConfigs[ti];
  return { llms: profile.llms, weights: profile.weights };
}

function getTaskWeight(profile: ProfileMeta | null | undefined, ti: number, total: number): number | null {
  if (!profile) return null;
  if (profile.kind === "tasks" && profile.taskWeights.length > ti) return profile.taskWeights[ti];
  return total > 0 ? 1 / total : null;
}

/* ── Main Component ── */

export function JudgmentStack({
  definition,
  execution,
  profile,
  resolvedSubFunctions,
  modelNames,
  depth = 0,
}: JudgmentStackProps) {
  const state = funcState(execution);
  const output = execution?.output ?? null;
  const reasoning = execution?.reasoning?.choices?.[0]?.message?.content ?? null;
  const tasks = definition?.tasks ?? [];
  const execTasks = execution?.tasks ?? [];
  const funcType = definition?.type ? normalizeType(definition.type) : null;
  const isRoot = depth === 0;

  const [expanded, setExpanded] = useState<Set<number>>(() => {
    if (tasks.length <= 4) return new Set(tasks.map((_, i) => i));
    return new Set([0]);
  });

  const toggle = (i: number) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      next.has(i) ? next.delete(i) : next.add(i);
      return next;
    });

  return (
    <div className={isRoot ? styles.stack : styles.nested}>
      {/* ── Header ── */}
      {isRoot && (
        <div className={styles.header}>
          <span className={styles.state} style={{ background: stateColor(state) }} />
          <span className={styles.name}>
            {execution?.function?.split("/").pop() ??
              (definition?.description
                ? definition.description.length > 48
                  ? definition.description.slice(0, 45) + "\u2026"
                  : definition.description
                : "Function")}
          </span>
          {funcType && (
            <span className={styles.typeBadge}>{funcType.replace(".function", "")}</span>
          )}
          {output != null && (
            <span className={styles.outputInline}>
              {Array.isArray(output)
                ? `#${output.indexOf(Math.max(...output)) + 1} \u00b7 ${pct(Math.max(...output))}`
                : pct(output as number)}
            </span>
          )}
          <span className={styles.headerMeta}>
            {tasks.length > 0 && <span>{tasks.length} tasks</span>}
            {execution?.id && <span>{execution.id}</span>}
          </span>
        </div>
      )}

      {/* ── Output distribution bar ── */}
      {Array.isArray(output) && isRoot && (
        <div className={styles.outputBar}>
          {(output as number[]).map((s, i) => (
            <div
              key={i}
              className={styles.outputSeg}
              style={{ flex: s, background: scoreColor(s), minWidth: s > 0 ? 2 : 0 }}
            />
          ))}
        </div>
      )}

      {/* ── Reasoning ── */}
      {reasoning && isRoot && (
        <div className={styles.reasoning}>
          <span className={styles.reasoningMark}>R</span>
          {reasoning.length > 140 ? reasoning.slice(0, 137) + "\u2026" : reasoning}
        </div>
      )}

      {/* ── Tasks ── */}
      {tasks.map((taskDef, ti) => (
        <TaskBand
          key={ti}
          index={ti}
          taskDef={taskDef}
          taskExec={execTasks[ti] ?? null}
          profile={profile}
          taskCount={tasks.length}
          expanded={expanded.has(ti)}
          onToggle={() => toggle(ti)}
          resolvedSubFunctions={resolvedSubFunctions}
          modelNames={modelNames}
          depth={depth}
        />
      ))}

      {/* ── Function convergence (multi-task) ── */}
      {tasks.length > 1 && output != null && isRoot && (
        <div className={styles.funcConvergence}>
          <span className={styles.convergenceLabel}>&Sigma; {tasks.length} tasks</span>
          <span className={styles.funcConvergenceOutput}>
            {Array.isArray(output) ? (output as number[]).map((s) => pct(s)).join("  ") : pct(output as number)}
          </span>
        </div>
      )}
    </div>
  );
}

/* ── Task Band ── */

function TaskBand({
  index: ti,
  taskDef,
  taskExec,
  profile,
  taskCount,
  expanded: isExpanded,
  onToggle,
  resolvedSubFunctions,
  modelNames,
  depth,
}: {
  index: number;
  taskDef: TaskDefinition;
  taskExec: TaskExecution | null;
  profile: ProfileMeta | null | undefined;
  taskCount: number;
  expanded: boolean;
  onToggle: () => void;
  resolvedSubFunctions?: Map<string, FunctionDefinition>;
  modelNames?: Record<string, string>;
  depth: number;
}) {
  const tState = taskState(taskExec);
  const taskW = getTaskWeight(profile, ti, taskCount);
  const agents = getTaskAgents(profile, ti);
  const type = shortType(taskDef.type);
  const nType = normalizeType(taskDef.type);
  const isVc = nType === "vector.completion";
  const isSubFn = nType.includes("function") && !nType.includes("placeholder");
  const prompt = promptPreview(taskDef);
  const responses = (taskDef.responses ?? []) as unknown[];
  const scores = taskExec?.scores ?? [];
  const votes = taskExec?.votes ?? [];

  const subFnKey =
    taskDef.owner && (taskDef.repository ?? taskDef.name)
      ? `${taskDef.owner}/${taskDef.repository ?? taskDef.name}`
      : null;
  const subFnDef = subFnKey ? resolvedSubFunctions?.get(subFnKey) ?? null : null;

  // Collect expression annotations
  const exprs: Array<{ key: string; value: string }> = [];
  const tryExpr = (key: string, obj: Record<string, unknown> | undefined) => {
    const s = exprStr(obj);
    if (s) exprs.push({ key, value: s });
  };
  tryExpr("skip", taskDef.skip);
  if (typeof taskDef.map === "object" && taskDef.map) {
    tryExpr("map", taskDef.map);
  } else if (taskDef.map != null) {
    exprs.push({ key: "map", value: String(taskDef.map) });
  }
  tryExpr("input", taskDef.input);
  tryExpr("output", taskDef.output);

  const agentCount = agents ? agents.llms.reduce((s, l) => s + l.count, 0) : 0;

  return (
    <div className={styles.task}>
      {/* Header — always visible */}
      <div className={styles.taskHeader} onClick={onToggle}>
        <span className={`${styles.arrow}${isExpanded ? ` ${styles.arrowOpen}` : ""}`}>
          &#x25B8;
        </span>
        <span className={styles.state} style={{ background: stateColor(tState) }} />
        <span className={styles.taskIndex}>{ti}</span>
        <span className={styles.taskType}>{type}</span>
        {taskW != null && <span className={styles.taskWeight}>&times;{taskW.toFixed(2)}</span>}
        {isSubFn && subFnKey && <span className={styles.taskRef}>{subFnKey}</span>}
        {isVc && (votes.length > 0 || agentCount > 0) && (
          <span className={styles.taskMeta}>
            {votes.length > 0 ? `${votes.length} votes` : `${agentCount} agents`}
          </span>
        )}
        {isVc && responses.length > 0 && (
          <span className={styles.taskMeta}>{responses.length} responses</span>
        )}
        {prompt && !isExpanded && <span className={styles.taskPrompt}>{prompt}</span>}
        {scores.length > 0 && !isExpanded && (
          <span className={styles.taskScoresInline}>
            [{scores.map((s) => dotPct(s)).join(" ")}]
          </span>
        )}
      </div>

      {/* Expanded content */}
      {isExpanded && (
        <div className={styles.taskBody}>
          {/* Expressions */}
          {exprs.length > 0 && (
            <div className={styles.expressions}>
              {exprs.map((e, i) => (
                <span key={i} className={styles.expr}>
                  <span className={styles.exprKey}>{e.key}:</span> {e.value}
                </span>
              ))}
            </div>
          )}

          {/* Prompt */}
          {prompt && <div className={styles.prompt}>{prompt}</div>}

          {/* VC with execution data → vote matrix */}
          {isVc && votes.length > 0 ? (
            <VoteMatrix
              votes={votes}
              scores={scores}
              responses={responses}
              agents={agents}
              modelNames={modelNames}
            />
          ) : isVc ? (
            /* VC structural → response tags + agent weight bars */
            <StructuralVc responses={responses} agents={agents} />
          ) : null}

          {/* Sub-function → recurse */}
          {isSubFn && (subFnDef || taskExec?.tasks) && (
            <JudgmentStack
              definition={subFnDef}
              execution={
                taskExec?.tasks ? { tasks: taskExec.tasks, output: taskExec.output } : null
              }
              profile={profile}
              resolvedSubFunctions={resolvedSubFunctions}
              modelNames={modelNames}
              depth={depth + 1}
            />
          )}

          {/* Streaming indicator */}
          {tState === "streaming" && !votes.length && (
            <div className={styles.streaming}>&#x258C; running&hellip;</div>
          )}
        </div>
      )}
    </div>
  );
}

/* ── Vote Matrix ── */

function VoteMatrix({
  votes,
  scores,
  responses,
  agents,
  modelNames,
}: {
  votes: Vote[];
  scores: number[];
  responses: unknown[];
  agents: { llms: ProfileLlm[]; weights: number[] } | null;
  modelNames?: Record<string, string>;
}) {
  const numR = scores.length || responses.length || (votes[0]?.vote.length ?? 0);
  const maxWeight = Math.max(...votes.map((v) => v.weight), 0) || 1;
  const totalWeight = votes.reduce((s, v) => s + v.weight, 0);
  const maxScore = scores.length > 0 ? Math.max(...scores) : 0;

  return (
    <div className={styles.matrixWrap}>
      <table className={styles.matrix}>
        <thead>
          <tr>
            <th className={styles.colWeight}>weight</th>
            <th className={styles.colAgent}>agent</th>
            {Array.from({ length: numR }, (_, ri) => {
              const isW = scores.length > 0 && scores[ri] === maxScore;
              return (
                <th
                  key={ri}
                  className={`${styles.colResponse}${isW ? " " + styles.winner : ""}`}
                >
                  {labelFor(responses[ri], ri)}
                </th>
              );
            })}
            <th className={styles.colFlags} />
          </tr>
        </thead>
        <tbody>
          {votes.map((vote, vi) => {
            const name = modelNames?.[vote.model] ?? vote.model;
            const shortName = name.includes("/") ? name.split("/").pop()! : name;
            const relW = maxWeight > 0 ? vote.weight / maxWeight : 0;
            const maxV = Math.max(...vote.vote);
            const llmDef = agents?.llms?.[vi];

            return (
              <tr
                key={vi}
                className={styles.agentRow}
                style={{
                  background:
                    relW > 0.7
                      ? `rgba(245, 158, 11, ${relW * 0.04})`
                      : "transparent",
                }}
              >
                {/* Weight */}
                <td className={styles.weightCell}>
                  <div className={styles.weightInner}>
                    <div className={styles.weightBar}>
                      <div
                        className={styles.weightFill}
                        style={{
                          height: `${relW * 100}%`,
                          background: `rgba(245, 158, 11, ${0.4 + relW * 0.5})`,
                        }}
                      />
                    </div>
                    <span className={styles.weightNum}>{vote.weight.toFixed(2)}</span>
                  </div>
                </td>
                {/* Agent */}
                <td className={styles.agentCell}>
                  <span className={styles.agentName}>{shortName}</span>
                  {llmDef?.outputMode && llmDef.outputMode !== "default" && (
                    <span className={styles.agentMode}>
                      {llmDef.outputMode === "log_probs"
                        ? "LP"
                        : llmDef.outputMode.slice(0, 3)}
                      {llmDef.topLogprobs ?? ""}
                    </span>
                  )}
                </td>
                {/* Votes */}
                {vote.vote.map((v, ri) => {
                  const isMax = v === maxV && v > 0;
                  const heat =
                    v > 0.01 ? `rgba(245, 158, 11, ${v * 0.3})` : "transparent";
                  const contribution =
                    totalWeight > 0 ? (v * vote.weight) / totalWeight : 0;
                  const scoreR = scores[ri] ?? 0;
                  const share = scoreR > 0 ? contribution / scoreR : 0;

                  return (
                    <td
                      key={ri}
                      className={styles.voteCell}
                      style={{ background: heat }}
                    >
                      <div
                        className={styles.voteNum}
                        style={{
                          color: isMax
                            ? "var(--info-bright)"
                            : v > 0.2
                              ? "var(--info-mid)"
                              : "var(--info-dim)",
                          fontWeight: isMax ? 600 : 400,
                        }}
                      >
                        {dotPct(v)}
                      </div>
                      {scores.length > 0 && (
                        <div className={styles.contributionTrack}>
                          <div
                            className={styles.contributionFill}
                            style={{
                              width: `${Math.min(100, share * 100)}%`,
                              background: isMax
                                ? "rgba(245, 158, 11, 0.6)"
                                : "rgba(120, 113, 108, 0.35)",
                            }}
                          />
                        </div>
                      )}
                    </td>
                  );
                })}
                {/* Flags */}
                <td className={styles.flagCell}>
                  {vote.from_cache && <span>C</span>}
                  {vote.from_rng && (
                    <span style={{ color: "var(--copper-mid)" }}>R</span>
                  )}
                </td>
              </tr>
            );
          })}

          {/* Convergence: Σ weighted */}
          {scores.length > 0 && (
            <tr className={styles.convergenceRow}>
              <td />
              <td className={styles.convergenceLabel}>&Sigma; weighted</td>
              {scores.map((score, ri) => {
                const isW = score === maxScore;
                return (
                  <td
                    key={ri}
                    className={styles.convergenceScore}
                    style={{
                      background: isW
                        ? "rgba(245, 158, 11, 0.1)"
                        : "transparent",
                    }}
                  >
                    <span
                      style={{
                        fontSize: isW ? 13 : 10,
                        fontWeight: isW ? 700 : 500,
                        color: isW ? "var(--info-bright)" : "var(--copper-dim)",
                      }}
                    >
                      {pct(score)}
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
  );
}

/* ── Structural VC (definition only, no execution) ── */

function StructuralVc({
  responses,
  agents,
}: {
  responses: unknown[];
  agents: { llms: ProfileLlm[]; weights: number[] } | null;
}) {
  const maxW = agents ? Math.max(...agents.weights, 0) || 1 : 1;

  return (
    <div>
      {responses.length > 0 && (
        <div className={styles.responses}>
          {responses.map((r, i) => (
            <span key={i} className={styles.responseTag}>
              {labelFor(r, i)}
            </span>
          ))}
        </div>
      )}

      {agents && agents.llms.length > 0 && (
        <div className={styles.structuralAgents}>
          {agents.llms.map((llm, i) => {
            const w = agents.weights[i] ?? 0;
            const rel = maxW > 0 ? w / maxW : 0;
            const name = llm.model.includes("/") ? llm.model.split("/").pop() : llm.model;
            return (
              <div key={i} className={styles.structuralAgent}>
                <div
                  className={styles.structuralAgentBar}
                  style={{
                    width: `${Math.max(20, rel * 80)}%`,
                    background: `rgba(245, 158, 11, ${0.2 + rel * 0.5})`,
                  }}
                />
                <span className={styles.structuralAgentName}>
                  {name}
                  {llm.count > 1 ? ` \u00d7${llm.count}` : ""}
                </span>
                <span className={styles.structuralAgentWeight}>{w.toFixed(2)}</span>
              </div>
            );
          })}
        </div>
      )}

      {!agents && responses.length === 0 && (
        <div className={styles.empty}>no configuration</div>
      )}
    </div>
  );
}
