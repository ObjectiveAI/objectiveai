import { useState, useCallback, useEffect } from "react";
import * as Collapsible from "@radix-ui/react-collapsible";
import { AgentCompletionChat } from "./components/shared/AgentCompletionChat";
import type { FunctionInventionRecursiveEntry } from "./types";

function CopyButton({ text, label }: { text: string; label?: string }) {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const t = setTimeout(() => setCopied(false), 1500);
    return () => clearTimeout(t);
  }, [copied]);

  const copy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
    } catch { /* clipboard denied */ }
  }, [text]);

  return (
    <button
      onClick={copy}
      className="text-[10px] text-info-dim hover:text-copper-bright transition-colors"
      title="Copy to clipboard"
    >
      {copied ? "copied" : (label ?? "copy")}
    </button>
  );
}

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
                  <span className="text-info-dim font-normal text-[11px] ml-auto flex items-center gap-1.5">
                    {inv.path.remote === "mock"
                      ? inv.path.name
                      : `${inv.path.owner}/${inv.path.repository}`}
                    {hasFunction && (
                      <CopyButton
                        text={inv.path.remote === "mock" ? inv.path.name : `${inv.path.owner}/${inv.path.repository}`}
                        label="copy path"
                      />
                    )}
                  </span>
                )}
              </div>
              {description && (
                <div className="text-xs text-info-mid mt-1">{description}</div>
              )}
            </div>

            {hasFunction && (
              <div className="max-w-content mx-auto mb-3 px-4">
                <Collapsible.Root defaultOpen={false}>
                  <div className="flex items-center gap-2">
                    <Collapsible.Trigger className="flex items-center gap-1 text-[11px] text-info-dim hover:text-info-mid cursor-pointer transition-colors group">
                      <svg
                        className="w-3 h-3 transition-transform group-data-[state=open]:rotate-90"
                        viewBox="0 0 12 12"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="1.5"
                      >
                        <path d="M4.5 2.5L8 6L4.5 9.5" />
                      </svg>
                      View function definition
                    </Collapsible.Trigger>
                    <CopyButton text={JSON.stringify(inv.function, null, 2)} />
                  </div>
                  <Collapsible.Content>
                    <div className="mt-2 bg-ground-surface border border-node-border rounded-md p-3 max-h-[300px] overflow-y-auto">
                      <pre className="font-mono text-[11px] text-info-mid whitespace-pre-wrap break-words">
                        {JSON.stringify(inv.function, null, 2)}
                      </pre>
                    </div>
                  </Collapsible.Content>
                </Collapsible.Root>
              </div>
            )}

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
