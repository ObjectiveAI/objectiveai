"use client";

import { useState } from "react";
import type { InventionStep } from "@/lib/demo-data";
import { INVENTION_STEPS } from "@/lib/demo-data";
import styles from "./InventionStream.module.css";

/* ── Types ── */

interface StepData {
  status: "complete" | "streaming" | "pending";
  text: string;
}

export interface InventionStreamProps {
  /** Function name being invented */
  name: string;
  /** Current active step (null if idle) */
  currentStep: InventionStep | null;
  /** Step data keyed by step name */
  steps: Record<InventionStep, StepData>;
  /** Overall state */
  state: "idle" | "streaming" | "done" | "error";
}

/* ── Step display labels ── */

const STEP_LABELS: Record<InventionStep, string> = {
  essay: "essay",
  input_schema: "schema",
  essay_tasks: "plan",
  tasks: "tasks",
  description: "description",
};

/* ── Component ── */

export function InventionStream({ name, currentStep, steps, state }: InventionStreamProps) {
  const [selectedStep, setSelectedStep] = useState<InventionStep | null>(null);

  // Show the selected step, or fall back to current streaming step, or first complete step
  const visibleStep = selectedStep ?? currentStep ?? findLastComplete(steps);

  const dotColor =
    state === "done" ? "var(--copper-hot)" :
    state === "streaming" ? "var(--copper-mid)" :
    state === "error" ? "var(--error, #ef4444)" :
    "var(--info-dim)";

  const stateLabel =
    state === "done" ? "complete" :
    state === "streaming" ? "inventing" :
    state === "error" ? "error" :
    "idle";

  return (
    <div className={styles.container}>
      {/* Header */}
      <div className={styles.header}>
        <span className={styles.headerName}>{name}</span>
        <span className={styles.headerState}>
          <span className={styles.stateDot} style={{ background: dotColor }} />
          {stateLabel}
        </span>
      </div>

      {/* Progress rail */}
      <div className={styles.rail}>
        {INVENTION_STEPS.map((step) => {
          const data = steps[step];
          const stepClass =
            data.status === "complete" ? styles.stepComplete :
            data.status === "streaming" ? styles.stepActive :
            styles.stepPending;

          return (
            <button
              key={step}
              className={`${styles.step} ${stepClass}`}
              onClick={() => setSelectedStep(step)}
            >
              {STEP_LABELS[step]}
              {data.status === "complete" && (
                <span className={styles.completeIndicator}>&#x2713;</span>
              )}
            </button>
          );
        })}
      </div>

      {/* Step content */}
      <div className={styles.content}>
        {visibleStep && (
          <StepContent step={visibleStep} data={steps[visibleStep]} />
        )}
      </div>
    </div>
  );
}

/* ── Step content renderer ── */

function StepContent({ step, data }: { step: InventionStep; data: StepData }) {
  if (data.status === "pending") {
    return (
      <div className={styles.stepContent}>
        <span className={styles.pendingText}>waiting for {STEP_LABELS[step]}...</span>
      </div>
    );
  }

  return (
    <div className={styles.stepContent}>
      <div className={styles.streamText}>
        {data.text}
        {data.status === "streaming" && <span className={styles.cursor} />}
      </div>
    </div>
  );
}

/* ── Helpers ── */

function findLastComplete(steps: Record<InventionStep, StepData>): InventionStep | null {
  for (let i = INVENTION_STEPS.length - 1; i >= 0; i--) {
    if (steps[INVENTION_STEPS[i]].status === "complete") {
      return INVENTION_STEPS[i];
    }
  }
  return INVENTION_STEPS[0];
}
