import type {
  FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk,
} from "objectiveai";
import { AgentCompletionChat } from "./components/shared/AgentCompletionChat";

interface FunctionInventionRecursiveEntry {
  kind: "invention";
  id: string;
  request: { id: string };
  chunk: FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk | null;
  error: { id: string; code: number; message: unknown } | null;
}

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
            <div className="max-w-[800px] mx-auto mt-4 mb-2 px-4 pb-2.5 font-mono text-[13px] font-bold text-info-bright border-b-2 border-info-dim">
              {name}
              {inv.path && (
                <span className="text-info-dim font-normal ml-2">
                  {inv.path.remote === "filesystem"
                    ? `filesystem:${inv.path.owner}/${inv.path.repository}`
                    : JSON.stringify(inv.path)}
                </span>
              )}
            </div>

            {inv.completions.length === 0 && !invError && (
              <div className="max-w-[800px] mx-auto mb-3 text-info-dim italic px-4">
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

            {invError && (
              <div className="max-w-[800px] mx-auto mb-4 bg-error/10 border border-error/30 rounded-md px-4 py-2 text-error text-xs">
                Invention error {invError.code}: {JSON.stringify(invError.message)}
              </div>
            )}
          </div>
        );
      })}

      {!chunk && !topError && (
        <div className="max-w-[800px] mx-auto mb-6 p-4 text-info-dim italic text-center">
          Waiting for invention…
        </div>
      )}

      {topError && (
        <div className="max-w-[800px] mx-auto mb-6 bg-error/10 border border-error/30 rounded-md px-4 py-2 text-error text-xs">
          Error {topError.code}: {JSON.stringify(topError.message)}
        </div>
      )}
    </div>
  );
}
