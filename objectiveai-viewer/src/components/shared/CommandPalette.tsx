import { useState, useRef, useEffect } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { useCliCommand } from "../../hooks/useCliCommand";

const QUICK_ACTIONS = [
  { label: "Execute function", args: "functions executions create" },
  { label: "Invent function", args: "functions inventions recursive create" },
  { label: "List functions", args: "functions list" },
  { label: "List agents", args: "agents list" },
];

export function CommandPalette({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [input, setInput] = useState("");
  const { lines, isRunning, run, clear } = useCliCommand();
  const inputRef = useRef<HTMLInputElement>(null);
  const outputRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (open) {
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight;
    }
  }, [lines]);

  const submit = () => {
    const trimmed = input.trim();
    if (!trimmed || isRunning) return;
    const args = trimmed.split(/\s+/);
    run(args);
  };

  const handleQuickAction = (argsStr: string) => {
    setInput(argsStr);
    run(argsStr.split(/\s+/));
  };

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/60 z-50" />
        <Dialog.Content className="fixed top-[15%] left-1/2 -translate-x-1/2 w-[90vw] max-w-lg z-50 bg-ground-raised border border-node-border rounded-lg shadow-2xl overflow-hidden">
          <div className="p-4 border-b border-node-border">
            <Dialog.Title className="text-xs font-mono text-info-dim uppercase tracking-wide mb-3">
              Command Palette
            </Dialog.Title>
            <div className="flex gap-1.5 mb-3 flex-wrap">
              {QUICK_ACTIONS.map((action) => (
                <button
                  key={action.label}
                  onClick={() => handleQuickAction(action.args)}
                  disabled={isRunning}
                  className="px-2 py-1 rounded-sm text-[10px] font-mono bg-ground-surface text-info-mid hover:text-copper-bright hover:bg-copper-warm/10 transition-colors disabled:opacity-50"
                >
                  {action.label}
                </button>
              ))}
            </div>
            <div className="flex gap-2">
              <input
                ref={inputRef}
                type="text"
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => { if (e.key === "Enter") submit(); }}
                placeholder="objectiveai <args...>"
                className="flex-1 bg-ground-surface border border-node-border rounded-sm px-3 py-1.5 text-xs font-mono text-info-bright placeholder:text-info-dim/50 outline-none focus:border-copper-dim"
              />
              <button
                onClick={submit}
                disabled={isRunning || !input.trim()}
                className="px-3 py-1.5 rounded-sm text-xs font-mono bg-copper-warm/20 text-copper-bright hover:bg-copper-warm/30 transition-colors disabled:opacity-50"
              >
                {isRunning ? "…" : "Run"}
              </button>
            </div>
          </div>

          {lines.length > 0 && (
            <div
              ref={outputRef}
              className="max-h-64 overflow-y-auto p-3 font-mono text-[11px] leading-relaxed bg-ground-base"
            >
              {lines.map((line, i) => (
                <div
                  key={i}
                  className={
                    line.type === "error"
                      ? "text-error"
                      : line.type === "end"
                        ? "text-info-dim italic"
                        : "text-info-mid"
                  }
                >
                  {line.type === "end"
                    ? "— done —"
                    : typeof line.value === "string"
                      ? line.value
                      : line.value !== null
                        ? JSON.stringify(line.value)
                        : `[${line.type}]`}
                </div>
              ))}
              {isRunning && (
                <span className="inline-block w-1.5 h-3 bg-copper-bright animate-pulse" />
              )}
            </div>
          )}

          {lines.length > 0 && !isRunning && (
            <div className="px-3 py-2 border-t border-node-border">
              <button
                onClick={clear}
                className="text-[10px] font-mono text-info-dim hover:text-info-mid transition-colors"
              >
                Clear
              </button>
            </div>
          )}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
