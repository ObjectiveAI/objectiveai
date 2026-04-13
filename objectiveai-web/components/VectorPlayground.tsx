"use client";

import { useState, useCallback } from "react";
import { DEMO_VECTOR_COMPLETION, DEMO_MODEL_NAMES } from "@/lib/demo-data";
import styles from "./VectorPlayground.module.css";

/* ── Types ── */

interface DisplayVote {
  model: string;
  vote: number[];
  weight: number;
}

export interface VectorPlaygroundProps {
  /** Pre-fill prompt */
  initialPrompt?: string;
  /** Pre-fill responses */
  initialResponses?: string[];
}

type PlaygroundState = "idle" | "streaming" | "done";

/* ── Copper palette for score segments ── */

const SEGMENT_COLORS = [
  "var(--copper-hot)",
  "var(--copper-mid)",
  "var(--copper-dim)",
  "var(--info-dim)",
  "var(--ground-surface)",
];

/* ── Component ── */

export function VectorPlayground({ initialPrompt, initialResponses }: VectorPlaygroundProps) {
  const [prompt, setPrompt] = useState(initialPrompt ?? "");
  const [responses, setResponses] = useState<string[]>(initialResponses ?? ["", ""]);
  const [state, setState] = useState<PlaygroundState>("idle");
  const [scores, setScores] = useState<number[]>([]);
  const [votes, setVotes] = useState<DisplayVote[]>([]);

  const updateResponse = useCallback((idx: number, val: string) => {
    setResponses((prev) => {
      const next = [...prev];
      next[idx] = val;
      return next;
    });
  }, []);

  const addResponse = useCallback(() => {
    setResponses((prev) => [...prev, ""]);
  }, []);

  const removeResponse = useCallback((idx: number) => {
    setResponses((prev) => prev.filter((_, i) => i !== idx));
  }, []);

  const handleRun = useCallback(() => {
    // TODO: Replace with useVectorCompletion() when API is available
    setState("streaming");
    const t = setTimeout(() => {
      setScores(DEMO_VECTOR_COMPLETION.scores);
      setVotes(DEMO_VECTOR_COMPLETION.votes);
      setState("done");
    }, 600);
    return () => clearTimeout(t);
  }, []);

  const dotColor =
    state === "done" ? "var(--copper-hot)" :
    state === "streaming" ? "var(--copper-mid)" :
    "var(--info-dim)";

  const stateLabel =
    state === "done" ? "complete" :
    state === "streaming" ? "voting" :
    "ready";

  const canRun = prompt.trim().length > 0 && responses.filter((r) => r.trim()).length >= 2;

  return (
    <div className={styles.container}>
      {/* Header */}
      <div className={styles.header}>
        <span className={styles.headerTitle}>vector completion</span>
        <span className={styles.headerState}>
          <span className={styles.stateDot} style={{ background: dotColor }} />
          {stateLabel}
        </span>
      </div>

      {/* Prompt */}
      <div className={styles.inputArea}>
        <label className={styles.promptLabel}>prompt</label>
        <textarea
          className={styles.promptTextarea}
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          placeholder="Enter a prompt for the swarm to judge..."
          disabled={state === "streaming"}
        />
      </div>

      {/* Responses */}
      <div className={styles.responsesSection}>
        <span className={styles.responsesLabel}>responses</span>
        {responses.map((r, i) => (
          <div key={i} className={styles.responseRow}>
            <span className={styles.responseIndex}>{i}</span>
            <input
              className={styles.responseInput}
              type="text"
              value={r}
              onChange={(e) => updateResponse(i, e.target.value)}
              placeholder={`response ${i}`}
              disabled={state === "streaming"}
            />
            {responses.length > 2 && (
              <button
                className={styles.responseRemove}
                onClick={() => removeResponse(i)}
                disabled={state === "streaming"}
              >
                ×
              </button>
            )}
          </div>
        ))}
        <button
          className={styles.addResponse}
          onClick={addResponse}
          disabled={state === "streaming"}
        >
          + add response
        </button>
      </div>

      {/* Run */}
      <div className={styles.actions}>
        <button
          className={styles.runButton}
          onClick={handleRun}
          disabled={!canRun || state === "streaming"}
        >
          {state === "streaming" ? "voting..." : "run"}
        </button>
      </div>

      {/* Results */}
      <div className={styles.results}>
        {state === "idle" && (
          <div className={styles.emptyState}>
            configure prompt and responses, then run
          </div>
        )}

        {scores.length > 0 && (
          <>
            {/* Score bar */}
            <div className={styles.scoresBar}>
              {scores.map((s, i) => (
                <div
                  key={i}
                  className={styles.scoreSegment}
                  style={{
                    flex: Math.max(s, 0.02),
                    background: SEGMENT_COLORS[i % SEGMENT_COLORS.length],
                  }}
                >
                  {s >= 0.08 ? (s * 100).toFixed(0) + "%" : ""}
                </div>
              ))}
            </div>

            {/* Vote breakdown */}
            <div className={styles.voteList}>
              {votes.map((v, i) => (
                <div key={i} className={styles.voteRow}>
                  <span className={styles.voteModel}>
                    {DEMO_MODEL_NAMES[v.model] ?? v.model}
                  </span>
                  <div className={styles.voteBar}>
                    {v.vote.map((val, j) => (
                      <div
                        key={j}
                        className={styles.voteSegment}
                        style={{
                          flex: Math.max(val, 0.01),
                          background: SEGMENT_COLORS[j % SEGMENT_COLORS.length],
                        }}
                      />
                    ))}
                  </div>
                  <span className={styles.voteWeight}>
                    {v.weight.toFixed(1)}
                  </span>
                </div>
              ))}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
