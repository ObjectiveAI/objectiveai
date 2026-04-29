import "./VoteMatrix.css";
import { dotPct, pct, labelFor } from "./judgment-utils";

interface Vote {
  agent: string;
  vote: number[];
  weight: number;
  from_cache?: boolean | null;
  swarm_index?: number;
  flat_swarm_index?: number;
}

export function VoteMatrix({
  votes,
  scores,
  responses,
}: {
  votes: Vote[];
  scores: number[];
  responses?: unknown[];
}) {
  const numR = scores.length || (votes[0]?.vote.length ?? 0);
  const maxWeight = Math.max(...votes.map((v) => v.weight), 0) || 1;
  const totalWeight = votes.reduce((s, v) => s + v.weight, 0);
  const maxScore = scores.length > 0 ? Math.max(...scores) : 0;

  return (
    <div className="vm-wrap">
      <table className="vm-table">
        <thead>
          <tr>
            <th className="vm-col-weight">weight</th>
            <th className="vm-col-agent">agent</th>
            {Array.from({ length: numR }, (_, ri) => {
              const isW = scores.length > 0 && scores[ri] === maxScore;
              return (
                <th
                  key={ri}
                  className={`vm-col-response${isW ? " vm-winner" : ""}`}
                >
                  {responses ? labelFor(responses[ri], ri) : `#${ri + 1}`}
                </th>
              );
            })}
            <th className="vm-col-flags" />
          </tr>
        </thead>
        <tbody>
          {votes.map((vote, vi) => {
            const shortName = vote.agent.length > 16
              ? vote.agent.slice(0, 14) + "…"
              : vote.agent;
            const relW = maxWeight > 0 ? vote.weight / maxWeight : 0;
            const maxV = Math.max(...vote.vote);

            return (
              <tr
                key={vi}
                className="vm-agent-row"
                style={{ "--row-alpha": relW > 0.7 ? relW * 0.04 : 0 } as React.CSSProperties}
              >
                <td className="vm-weight-cell">
                  <div className="vm-weight-inner">
                    <div className="vm-weight-bar">
                      <div
                        className="vm-weight-fill"
                        style={{ height: `${relW * 100}%`, "--fill-alpha": 0.4 + relW * 0.5 } as React.CSSProperties}
                      />
                    </div>
                    <span className="vm-weight-num">{vote.weight.toFixed(2)}</span>
                  </div>
                </td>
                <td className="vm-agent-cell">
                  <span className="vm-agent-name">{shortName}</span>
                </td>
                {vote.vote.map((v, ri) => {
                  const isMax = v === maxV && v > 0;
                  const contribution =
                    totalWeight > 0 ? (v * vote.weight) / totalWeight : 0;
                  const scoreR = scores[ri] ?? 0;
                  const share = scoreR > 0 ? contribution / scoreR : 0;

                  return (
                    <td
                      key={ri}
                      className="vm-vote-cell"
                      style={{ "--heat-alpha": v > 0.01 ? v * 0.3 : 0 } as React.CSSProperties}
                    >
                      <div
                        className={`vm-vote-num ${isMax ? "vm-vote-max" : v > 0.2 ? "vm-vote-mid" : "vm-vote-dim"}`}
                      >
                        {dotPct(v)}
                      </div>
                      {scores.length > 0 && (
                        <div className="vm-contribution-track">
                          <div
                            className={`vm-contribution-fill${isMax ? " vm-contribution-winner" : ""}`}
                            style={{ width: `${Math.min(100, share * 100)}%` }}
                          />
                        </div>
                      )}
                    </td>
                  );
                })}
                <td className="vm-flag-cell">
                  {vote.from_cache && <span>C</span>}
                </td>
              </tr>
            );
          })}

          {scores.length > 0 && (
            <tr className="vm-convergence-row">
              <td />
              <td className="vm-convergence-label">&Sigma; weighted</td>
              {scores.map((score, ri) => {
                const isW = score === maxScore;
                return (
                  <td
                    key={ri}
                    className={`vm-convergence-score${isW ? " vm-convergence-winner" : ""}`}
                  >
                    <span className={`vm-convergence-value${isW ? " vm-convergence-value-winner" : ""}`}>
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
