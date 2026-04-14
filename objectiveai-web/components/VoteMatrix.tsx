import type { ProfileLlm } from "@/lib/profiles/types";
import type { Vote } from "@/lib/judgment-stack-utils";
import { dotPct, pct, scoreColor, labelFor } from "@/lib/judgment-stack-utils";
import styles from "./VoteMatrix.module.css";

export function VoteMatrix({
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
