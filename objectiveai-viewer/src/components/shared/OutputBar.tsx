import cn from "classnames";
import { scoreColor, formatScore } from "../../lib/scoring";
import { JsonBlock } from "./JsonBlock";

function truncateLabel(label: string, max = 60): string {
  if (label.length <= max) return label;
  return label.slice(0, max - 1) + "…";
}

export function OutputBar({ output, labels }: { output: unknown; labels?: string[] }) {
  if (output === undefined || output === null) return null;

  if (typeof output === "number") {
    return (
      <div className={cn("flex", "items-center", "gap-2")}>
        <div className={cn("flex-1", "h-1.5", "bg-ground-surface", "rounded-full", "overflow-hidden")}>
          <div
            className={cn("h-full", "rounded-full")}
            style={{ width: `${output * 100}%`, background: scoreColor(output) }}
          />
        </div>
        <span className={cn("font-mono", "text-[10px]", "text-info-dim", "tabular-nums")}>{formatScore(output)}</span>
      </div>
    );
  }

  if (Array.isArray(output) && output.length > 0 && output.every((v) => typeof v === "number")) {
    const scores = output as number[];
    const hasLabels = labels && labels.length === scores.length;
    return (
      <div className={cn("space-y-1")}>
        <div className={cn("relative", "group")}>
          <div className={cn("flex", "h-1.5", "gap-px", "rounded-full", "overflow-hidden")}>
            {scores.map((s, i) => (
              <div
                key={i}
                className={cn("h-full", "first:rounded-l-full", "last:rounded-r-full")}
                style={{ flex: Math.max(s, 0.005), background: scoreColor(s) }}
              />
            ))}
          </div>
          {hasLabels && (
            <div className={cn("absolute", "left-0", "top-full", "mt-1", "z-10", "hidden", "group-hover:block", "bg-ground-base", "border", "border-node-border", "rounded", "px-2", "py-1.5", "shadow-lg", "max-w-[400px]")}>
              {scores.map((s, i) => (
                <div key={i} className={cn("flex", "items-center", "gap-2", "py-0.5")}>
                  <div className={cn("w-2", "h-2", "rounded-sm", "flex-shrink-0")} style={{ background: scoreColor(s) }} />
                  <span className={cn("font-mono", "text-[10px]", "text-info-mid", "truncate")}>{truncateLabel(labels[i])}</span>
                  <span className={cn("font-mono", "text-[10px]", "text-info-dim", "tabular-nums", "ml-auto", "pl-2")}>{formatScore(s)}</span>
                </div>
              ))}
            </div>
          )}
        </div>
        <div className={cn("flex", "flex-col", "gap-0.5")}>
          {scores.map((s, i) => (
            <div key={i} className={cn("flex", "items-center", "gap-2")}>
              {hasLabels ? (
                <span
                  className={cn("font-mono", "text-[10px]", "text-info-mid", "truncate", "max-w-[240px]", "flex-shrink-0")}
                  title={labels[i]}
                >
                  {truncateLabel(labels[i])}
                </span>
              ) : (
                <span className={cn("font-mono", "text-[10px]", "text-info-dim", "tabular-nums", "flex-shrink-0")}>
                  {i}:
                </span>
              )}
              <div className={cn("flex-1", "h-1", "bg-ground-surface", "rounded-full", "overflow-hidden", "min-w-[60px]")}>
                <div
                  className={cn("h-full", "rounded-full")}
                  style={{ width: `${s * 100}%`, background: scoreColor(s) }}
                />
              </div>
              <span className={cn("font-mono", "text-[10px]", "tabular-nums", "flex-shrink-0")} style={{ color: scoreColor(s) }}>
                {formatScore(s)}
              </span>
            </div>
          ))}
        </div>
      </div>
    );
  }

  // Fallback for non-numeric output (objects, strings, mixed arrays)
  return (
    <JsonBlock
      value={output}
      className={cn("text-[11px]", "text-info-mid", "max-h-[200px]", "overflow-y-auto")}
    />
  );
}
