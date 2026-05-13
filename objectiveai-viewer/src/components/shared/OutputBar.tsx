import { scoreColor, formatScore } from "../../lib/scoring";

export function OutputBar({ output }: { output: unknown }) {
  if (output === undefined || output === null) return null;

  if (typeof output === "number") {
    return (
      <div className="flex items-center gap-2">
        <div className="flex-1 h-1.5 bg-ground-surface rounded-full overflow-hidden">
          <div
            className="h-full rounded-full"
            style={{ width: `${output * 100}%`, background: scoreColor(output) }}
          />
        </div>
        <span className="font-mono text-[10px] text-info-dim tabular-nums">{formatScore(output)}</span>
      </div>
    );
  }

  if (Array.isArray(output) && output.length > 0 && output.every((v) => typeof v === "number")) {
    const scores = output as number[];
    return (
      <div className="space-y-1">
        <div className="flex h-1.5 gap-px rounded-full overflow-hidden">
          {scores.map((s, i) => (
            <div
              key={i}
              className="h-full first:rounded-l-full last:rounded-r-full"
              style={{ flex: Math.max(s, 0.005), background: scoreColor(s) }}
            />
          ))}
        </div>
        <div className="flex gap-2 flex-wrap">
          {scores.map((s, i) => (
            <span key={i} className="font-mono text-[10px] tabular-nums" style={{ color: scoreColor(s) }}>
              {i}: {formatScore(s)}
            </span>
          ))}
        </div>
      </div>
    );
  }

  return null;
}
