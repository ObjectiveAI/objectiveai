import { AgentCompletionChat } from "./components/shared/AgentCompletionChat";
import type { FunctionInventionRecursiveEntry } from "./types";

function stateField(state: unknown, field: string): string | undefined {
  if (state && typeof state === "object" && field in state) {
    const v = (state as Record<string, unknown>)[field];
    if (typeof v === "string" && v.length > 0) return v;
  }
  return undefined;
}

function inventionName(state: unknown, fallbackIndex: number): string {
  return stateField(state, "name") ?? `invention #${fallbackIndex}`;
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

        const description = stateField(inv.state, "description");
        const fnType = stateField(inv.state, "type");
        const hasFunction = inv.function != null;

        return (
          <div key={inv.index ?? invIdx}>
            <div className="max-w-content mx-auto mt-4 mb-2 px-4 pb-2.5 border-b-2 border-info-dim">
              <div className="flex items-center gap-2 font-mono text-[13px] font-bold text-info-bright">
                {name}
                {fnType && (
                  <span className="text-[10px] font-normal text-copper-dim bg-ground-surface px-1.5 py-px rounded-sm">{fnType}</span>
                )}
                {hasFunction && (
                  <span className="text-[10px] font-semibold text-copper-hot bg-copper-hot/15 px-1.5 py-px rounded-sm">created</span>
                )}
                {inv.path && (
                  <span className="text-info-dim font-normal text-[11px] ml-auto">
                    {inv.path.remote === "filesystem"
                      ? `${inv.path.owner}/${inv.path.repository}`
                      : JSON.stringify(inv.path)}
                  </span>
                )}
              </div>
              {description && (
                <div className="text-xs text-info-mid mt-1">{description}</div>
              )}
            </div>

            {inv.completions.length === 0 && !invError && (
              <div className="max-w-content mx-auto mb-3 text-info-dim italic px-4">
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
              <div role="alert" className="max-w-content mx-auto mb-4 bg-error/10 border border-error/30 rounded-md px-4 py-2 text-error text-xs">
                Invention error {invError.code}: {JSON.stringify(invError.message)}
              </div>
            )}
          </div>
        );
      })}

      {!chunk && !topError && (
        <div className="max-w-content mx-auto mb-6 p-4 text-info-dim italic text-center">
          Waiting for invention…
        </div>
      )}

      {topError && (
        <div role="alert" className="max-w-content mx-auto mb-6 bg-error/10 border border-error/30 rounded-md px-4 py-2 text-error text-xs">
          Error {topError.code}: {JSON.stringify(topError.message)}
        </div>
      )}
    </div>
  );
}
