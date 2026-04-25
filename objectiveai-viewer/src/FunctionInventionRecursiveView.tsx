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
                <span style={{ color: "#888", fontWeight: 400, marginLeft: 8 }}>
                  {inv.path.remote === "filesystem"
                    ? `filesystem:${inv.path.owner}/${inv.path.repository}`
                    : JSON.stringify(inv.path)}
                </span>
              )}
            </div>

            {/* One chat per agent completion inside this invention. */}
            {inv.completions.length === 0 && !invError && (
              <div style={{ maxWidth: 800, margin: "0 auto 12px", color: "#999", fontStyle: "italic", padding: "0 16px" }}>
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
                style={{
                  maxWidth: 800,
                  margin: "0 auto 16px",
                  border: "1px solid #f5c6cb",
                  borderRadius: 8,
                }}
              >
                Invention error {invError.code}: {JSON.stringify(invError.message)}
              </div>
            )}
          </div>
        );
      })}

      {/* Placeholder when nothing has arrived yet. */}
      {!chunk && !topError && (
        <div
          style={{
            maxWidth: 800,
            margin: "0 auto 24px",
            padding: 16,
            color: "#999",
            fontStyle: "italic",
            textAlign: "center",
          }}
        >
          Waiting for invention…
        </div>
      )}

      {/* Top-level error for the whole recursive invention. */}
      {topError && (
        <div
          className="ac-error-banner"
          style={{
            maxWidth: 800,
            margin: "0 auto 24px",
            border: "1px solid #f5c6cb",
            borderRadius: 8,
          }}
        >
          Error {topError.code}: {JSON.stringify(topError.message)}
        </div>
      )}
    </div>
  );
}
