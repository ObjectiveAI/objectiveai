"use client";

import { useState } from "react";
import type { FunctionDefinition, TaskDefinition } from "@/lib/functions/types";
import type { ProfileMeta } from "@/lib/profiles/types";
import type { FunctionExecution, TaskExecution } from "@/lib/judgment-stack-utils";
import {
  scoreColor,
  pct,
  dotPct,
  stateColor,
  taskState,
  funcState,
  normalizeType,
  shortType,
  exprStr,
  promptPreview,
  getTaskAgents,
  getTaskWeight,
} from "@/lib/judgment-stack-utils";
import { VoteMatrix } from "./VoteMatrix";
import { StructuralVc } from "./StructuralVc";
import { ConvergenceTimeline } from "./ConvergenceTimeline";
import styles from "./JudgmentStack.module.css";

export type { FunctionExecution } from "@/lib/judgment-stack-utils";

/* ── Props ── */

export interface JudgmentStackProps {
  definition?: FunctionDefinition | null;
  execution?: FunctionExecution | null;
  profile?: ProfileMeta | null | undefined;
  resolvedSubFunctions?: Map<string, FunctionDefinition>;
  modelNames?: Record<string, string>;
  depth?: number;
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
  const completions = (taskExec?.completions ?? []) as Array<Record<string, unknown>>;

  const [expandedAgent, setExpandedAgent] = useState<number | null>(null);

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

  const hasCompletionText = completions.some((c) => typeof c.text === "string" && c.text);

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

          {/* VC with execution data → convergence + vote matrix + agent text */}
          {isVc && votes.length > 0 ? (
            <>
              {votes.length >= 2 && (
                <ConvergenceTimeline
                  votes={votes}
                  responses={responses}
                  modelNames={modelNames}
                  compact
                />
              )}
              <VoteMatrix
                votes={votes}
                scores={scores}
                responses={responses}
                agents={agents}
                modelNames={modelNames}
              />
              {hasCompletionText && (
                <div className={styles.deliberation}>
                  <div className={styles.deliberationHeader}>
                    {votes.map((v, vi) => {
                      const name = modelNames?.[v.model] ?? v.model;
                      const short = name.includes("/") ? name.split("/").pop()! : name;
                      return (
                        <button
                          key={vi}
                          className={`${styles.deliberationTab}${expandedAgent === vi ? ` ${styles.deliberationTabActive}` : ""}`}
                          onClick={() => setExpandedAgent(expandedAgent === vi ? null : vi)}
                        >
                          {short}
                        </button>
                      );
                    })}
                  </div>
                  {expandedAgent != null && completions[expandedAgent] && (
                    <div className={styles.deliberationText}>
                      {String(completions[expandedAgent].text ?? "")}
                    </div>
                  )}
                </div>
              )}
            </>
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
