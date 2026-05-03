import type { Vote } from "@/lib/judgment-stack-utils";
import { labelFor } from "@/lib/judgment-stack-utils";
import styles from "./ConvergenceTimeline.module.css";

const COLORS = [
  "var(--copper-hot)",
  "var(--copper-mid)",
  "var(--copper-warm)",
  "var(--copper-dim)",
  "var(--info-dim)",
  "var(--info-mid)",
];

function computeConvergence(votes: Vote[]): number[][] {
  const history: number[][] = [];
  if (votes.length === 0) return history;
  const numR = votes[0].vote.length;
  for (let i = 0; i < votes.length; i++) {
    const slice = votes.slice(0, i + 1);
    const totalW = slice.reduce((s, v) => s + v.weight, 0);
    history.push(
      Array.from({ length: numR }, (_, r) =>
        totalW > 0 ? slice.reduce((s, v) => s + v.vote[r] * v.weight, 0) / totalW : 0
      )
    );
  }
  return history;
}

const LABEL_H = 9;

function decollideLabelYs(positions: { y: number; r: number }[]): Map<number, number> {
  const sorted = [...positions].sort((a, b) => a.y - b.y);
  for (let i = 1; i < sorted.length; i++) {
    if (sorted[i].y - sorted[i - 1].y < LABEL_H) {
      sorted[i].y = sorted[i - 1].y + LABEL_H;
    }
  }
  return new Map(sorted.map((p) => [p.r, p.y]));
}

export function ConvergenceTimeline({
  votes,
  responses,
  modelNames,
  compact = false,
}: {
  votes: Vote[];
  responses: unknown[];
  modelNames?: Record<string, string>;
  compact?: boolean;
}) {
  if (votes.length === 0) return null;

  const history = computeConvergence(votes);
  const numR = history[0]?.length ?? 0;
  const N = history.length;

  const W = 300;
  const H = compact ? 80 : 140;
  const pad = { left: 26, right: 68, top: 10, bottom: 16 };
  const chartH = H - pad.top - pad.bottom;
  const chartW = W - pad.left - pad.right;

  const px = (i: number) => pad.left + (N > 1 ? (i / (N - 1)) * chartW : chartW / 2);
  const py = (score: number) => pad.top + (1 - score) * chartH;

  const labelPositions = decollideLabelYs(
    Array.from({ length: numR }, (_, r) => ({ y: py(history[N - 1][r]) + 3, r }))
  );

  return (
    <div className={`${styles.wrap} ${compact ? styles.compact : ""}`}>
      <svg viewBox={`0 0 ${W} ${H}`} className={styles.svg} preserveAspectRatio="xMidYMid meet">
        {/* Guide lines */}
        <line x1={pad.left} y1={py(1)} x2={W - pad.right} y2={py(1)} className={styles.guide} />
        <line x1={pad.left} y1={py(0.5)} x2={W - pad.right} y2={py(0.5)} className={styles.guide} />
        <line x1={pad.left} y1={py(0)} x2={W - pad.right} y2={py(0)} className={styles.guide} />

        {/* Y-axis labels */}
        <text x={pad.left - 4} y={py(1) + 3} className={styles.axisLabel} textAnchor="end">1</text>
        <text x={pad.left - 4} y={py(0.5) + 3} className={styles.axisLabel} textAnchor="end">.5</text>
        <text x={pad.left - 4} y={py(0) + 3} className={styles.axisLabel} textAnchor="end">0</text>

        {/* Response lines */}
        {Array.from({ length: numR }, (_, r) => {
          const color = COLORS[r % COLORS.length];
          const points = history.map((h, i) => `${px(i)},${py(h[r])}`).join(" ");
          const final = history[N - 1][r];
          const label = labelFor(responses[r], r);
          const labelY = labelPositions.get(r) ?? py(final) + 3;

          return (
            <g key={r}>
              <polyline
                points={points}
                fill="none"
                stroke={color}
                strokeWidth={1.5}
                strokeLinejoin="round"
                strokeLinecap="round"
              />
              {history.map((h, i) => {
                const agent = modelNames?.[votes[i].model] ?? votes[i].model;
                return (
                  <circle key={i} cx={px(i)} cy={py(h[r])} r={2} fill={color} className={styles.dot}>
                    <title>{`${agent}\n${label}: ${(h[r] * 100).toFixed(1)}%`}</title>
                  </circle>
                );
              })}
              <text
                x={W - pad.right + 6}
                y={labelY}
                className={styles.endLabel}
                fill={color}
              >
                {label.length > 10 ? label.slice(0, 8) + "…" : label}{" "}
                .{(final * 100).toFixed(0).padStart(2, "0")}
              </text>
            </g>
          );
        })}

        {/* X-axis: vote count */}
        {history.map((_, i) => (
          <text key={i} x={px(i)} y={H - 2} className={styles.axisLabel} textAnchor="middle">
            {i + 1}
          </text>
        ))}
      </svg>
    </div>
  );
}
