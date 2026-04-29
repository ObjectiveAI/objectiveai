import type {
  FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk,
} from "objectiveai";
import { AgentCompletionChat } from "./AgentCompletionView";

interface FunctionInventionRecursiveEntry {
  kind: "invention";
  id: string;
  request: { id: string };
  chunk: FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk | null;
  error: { id: string; code: number; message: unknown } | null;
}

/// Name extractor for the invention's state.
/// State is `{ type: "...", ... }` with `name` flattened from params.
function inventionName(state: unknown, fallbackIndex: number): string {
  if (state && typeof state === "object" && "name" in state) {
    const n = (state as { name?: unknown }).name;
    if (typeof n === "string" && n.length > 0) return n;
  }
  return `invention #${fallbackIndex}`;
}

export function FunctionInventionRecursiveView({
  entry,
}: {
  entry: FunctionInventionRecursiveEntry;
}) {
  const chunk = entry.chunk;

  // Top-level error (applies to the whole recursive invention).
  const topError = entry.error
    ? { code: entry.error.code, message: entry.error.message }
    : null;

  return (
    <div>
      {chunk?.inventions.map((inv, invIdx) => {
        const name = inventionName(inv.state, inv.index ?? invIdx);
        const invError = inv.error
          ? { code: inv.error.code, message: inv.error.message }
          : null;

        return (
          <div key={inv.index ?? invIdx}>
            <div className="ac-section-header">
              {name}
              {inv.path && (
                <span style={{ color: "var(--info-dim)", fontWeight: 400, marginLeft: 8 }}>
                  {inv.path.remote === "filesystem"
                    ? `filesystem:${inv.path.owner}/${inv.path.repository}`
                    : JSON.stringify(inv.path)}
                </span>
              )}
            </div>

            {/* One chat per agent completion inside this invention. */}
            {inv.completions.length === 0 && !invError && (
              <div style={{ margin: "0 0 12px", color: "var(--info-dim)", fontFamily: "var(--font-mono)", fontSize: 11, padding: "0 16px" }}>
                No completions yet…
              </div>
            )}
            {inv.completions.map((comp, compIdx) => {
              const compError = comp.error
                ? { code: comp.error.code, message: comp.error.message }
                : null;
              const label = `${name} — completion #${comp.index ?? compIdx}`;
              return (
                <AgentCompletionChat
                  key={`${inv.index ?? invIdx}-${comp.index ?? compIdx}`}
                  label={label}
                  chunk={comp}
                  error={compError}
                  id={comp.id}
                />
              );
            })}

            {/* Per-invention error (distinct from per-completion errors). */}
            {invError && (
              <div
                className="ac-error-banner"
                style={{ margin: "0 0 16px" }}
              >
                Invention error {invError.code}: {JSON.stringify(invError.message)}
              </div>
            )}
          </div>
        );
      })}

      {/* Placeholder when nothing has arrived yet. */}
      {!chunk && !topError && (
        <div className="viewer-empty">
          Waiting for invention...
        </div>
      )}

      {/* Top-level error for the whole recursive invention. */}
      {topError && (
        <div
          className="ac-error-banner"
          style={{ margin: "0 0 24px" }}
        >
          Error {topError.code}: {JSON.stringify(topError.message)}
        </div>
      )}
    </div>
  );
}
