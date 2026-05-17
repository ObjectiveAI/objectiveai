import { useState, useRef, useEffect, useCallback } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { useCliCommand, type CliLine } from "../../hooks/useCliCommand";

interface GuidedMode {
  step: "picking" | "input";
  functions?: { id: string; type?: string }[];
  selectedFunction?: string;
  inputJson: string;
  error?: string;
}

const QUICK_ACTIONS: { label: string; args: string; guided?: boolean }[] = [
  { label: "Execute function", args: "functions executions create", guided: true },
  { label: "Invent function", args: "functions inventions recursive create" },
  { label: "List functions", args: "functions list" },
  { label: "List agents", args: "agents list" },
];

const MAX_HISTORY = 50;

// Module-level history persists across mount/unmount but not app restart
const commandHistory: string[] = [];

/** Attempt to detect and parse a JSON string */
function tryParseJson(text: string): unknown | null {
  const trimmed = text.trim();
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) return null;
  try {
    return JSON.parse(trimmed);
  } catch {
    return null;
  }
}

/** Render a JSON value with syntax coloring */
function JsonHighlighted({ value }: { value: unknown }) {
  const json = JSON.stringify(value, null, 2);
  // Tokenize JSON for highlighting
  const lines = json.split("\n");
  return (
    <pre className="whitespace-pre-wrap">
      {lines.map((line, i) => (
        <div key={i}>{colorizeJsonLine(line)}</div>
      ))}
    </pre>
  );
}

function colorizeJsonLine(line: string): React.ReactNode {
  // Match JSON tokens: keys, string values, numbers, booleans, null
  const parts: React.ReactNode[] = [];
  let remaining = line;
  let key = 0;

  while (remaining.length > 0) {
    // Match leading whitespace/structural chars
    const structMatch = remaining.match(/^([\s,\[\]{}:]+)/);
    if (structMatch) {
      parts.push(<span key={key++} className="text-info-dim">{structMatch[1]}</span>);
      remaining = remaining.slice(structMatch[1].length);
      continue;
    }

    // Match a quoted string (could be key or value)
    const strMatch = remaining.match(/^("(?:[^"\\]|\\.)*")/);
    if (strMatch) {
      const str = strMatch[1];
      // Check if this is a key (followed by colon)
      const afterStr = remaining.slice(str.length);
      const isKey = /^\s*:/.test(afterStr);
      if (isKey) {
        parts.push(<span key={key++} className="text-copper-mid">{str}</span>);
      } else {
        parts.push(<span key={key++} className="text-info-mid">{str}</span>);
      }
      remaining = remaining.slice(str.length);
      continue;
    }

    // Match numbers
    const numMatch = remaining.match(/^(-?\d+\.?\d*(?:[eE][+-]?\d+)?)/);
    if (numMatch) {
      parts.push(<span key={key++} className="text-copper-bright">{numMatch[1]}</span>);
      remaining = remaining.slice(numMatch[1].length);
      continue;
    }

    // Match booleans and null
    const boolMatch = remaining.match(/^(true|false|null)/);
    if (boolMatch) {
      parts.push(<span key={key++} className="text-copper-dim">{boolMatch[1]}</span>);
      remaining = remaining.slice(boolMatch[1].length);
      continue;
    }

    // Fallback: take one character
    parts.push(<span key={key++} className="text-info-mid">{remaining[0]}</span>);
    remaining = remaining.slice(1);
  }

  return <>{parts}</>;
}

/** Render a single output line with optional JSON highlighting */
function OutputLine({ line }: { line: CliLine }) {
  if (line.type === "end") {
    return <div className="text-info-dim italic">— done —</div>;
  }

  if (line.type === "error") {
    const text = typeof line.value === "string" ? line.value : JSON.stringify(line.value);
    return <div className="text-error">{text}</div>;
  }

  const text =
    typeof line.value === "string"
      ? line.value
      : line.value !== null
        ? JSON.stringify(line.value)
        : `[${line.type}]`;

  // Try JSON highlighting
  const parsed = tryParseJson(text);
  if (parsed !== null) {
    return (
      <div className="text-info-mid">
        <JsonHighlighted value={parsed} />
      </div>
    );
  }

  return <div className="text-info-mid">{text}</div>;
}

/** Guided execution flow for "Execute function" */
function GuidedExecution({
  mode,
  isRunning,
  onSelect,
  onUpdateInput,
  onRun,
  onBack,
}: {
  mode: GuidedMode;
  isRunning: boolean;
  onSelect: (fnId: string) => void;
  onUpdateInput: (json: string) => void;
  onRun: () => void;
  onBack: () => void;
}) {
  if (mode.step === "picking") {
    return (
      <div className="mt-2">
        <div className="flex items-center justify-between mb-2">
          <span className="text-[10px] font-mono text-copper-dim uppercase tracking-wide">
            Step 1: Select function
          </span>
          <button
            onClick={onBack}
            className="text-[10px] font-mono text-info-dim hover:text-info-mid transition-colors"
          >
            Back
          </button>
        </div>

        {isRunning && (
          <div className="flex items-center gap-2 py-3">
            <span className="inline-block w-1.5 h-3 bg-copper-bright animate-pulse" />
            <span className="text-[11px] font-mono text-info-dim">Loading functions...</span>
          </div>
        )}

        {mode.error && (
          <div className="text-[11px] font-mono text-error bg-ground-surface rounded-sm p-2 mb-2">
            {mode.error}
          </div>
        )}

        {mode.functions && mode.functions.length > 0 && (
          <div className="flex flex-wrap gap-1.5 max-h-40 overflow-y-auto py-1">
            {mode.functions.map((fn) => (
              <button
                key={fn.id}
                onClick={() => onSelect(fn.id)}
                className="flex items-center gap-1.5 px-2 py-1.5 rounded-sm text-[11px] font-mono bg-ground-surface border border-node-border text-info-mid hover:text-copper-bright hover:border-copper-dim transition-colors text-left"
              >
                <span className="truncate max-w-[200px]">{fn.id}</span>
                {fn.type && (
                  <span className="text-[9px] px-1 py-0.5 rounded-sm bg-copper-warm/10 text-copper-dim shrink-0">
                    {fn.type}
                  </span>
                )}
              </button>
            ))}
          </div>
        )}

        {mode.functions && mode.functions.length === 0 && !mode.error && (
          <div className="text-[11px] font-mono text-info-dim py-2">
            No functions found.
          </div>
        )}

        <button
          onClick={onBack}
          className="mt-2 text-[10px] font-mono text-info-dim hover:text-copper-dim transition-colors underline underline-offset-2"
        >
          Skip — type manually
        </button>
      </div>
    );
  }

  // Step 2: Input editor
  const commandPreview = `functions executions create --function ${mode.selectedFunction}`;

  return (
    <div className="mt-2">
      <div className="flex items-center justify-between mb-2">
        <span className="text-[10px] font-mono text-copper-dim uppercase tracking-wide">
          Step 2: Configure input
        </span>
        <button
          onClick={() => onSelect("")}
          className="text-[10px] font-mono text-info-dim hover:text-info-mid transition-colors"
        >
          Back
        </button>
      </div>

      <div className="text-[10px] font-mono text-info-dim bg-ground-surface rounded-sm px-2 py-1.5 mb-2 border border-node-border/50">
        $ objectiveai {commandPreview}
      </div>

      <label className="block text-[10px] font-mono text-info-dim mb-1">
        Input JSON (optional)
      </label>
      <textarea
        value={mode.inputJson}
        onChange={(e) => onUpdateInput(e.target.value)}
        placeholder='{"key": "value"}'
        rows={4}
        className="w-full bg-ground-surface border border-node-border rounded-sm px-3 py-2 text-[11px] font-mono text-info-bright placeholder:text-info-dim/50 outline-none focus:border-copper-dim resize-y"
      />

      <div className="flex gap-2 mt-2">
        <button
          onClick={onRun}
          disabled={isRunning}
          className="px-3 py-1.5 rounded-sm text-xs font-mono bg-copper-warm/20 text-copper-bright hover:bg-copper-warm/30 transition-colors disabled:opacity-50"
        >
          {isRunning ? "..." : "Run"}
        </button>
        <button
          onClick={onBack}
          className="px-3 py-1.5 rounded-sm text-xs font-mono text-info-dim hover:text-info-mid transition-colors"
        >
          Cancel
        </button>
      </div>
    </div>
  );
}

export function CommandPalette({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [input, setInput] = useState("");
  const [lastCommand, setLastCommand] = useState<string | null>(null);
  const [guidedMode, setGuidedMode] = useState<GuidedMode | null>(null);
  const { lines, isRunning, run, clear } = useCliCommand();
  const inputRef = useRef<HTMLInputElement>(null);
  const outputRef = useRef<HTMLDivElement>(null);
  const historyIndexRef = useRef<number>(-1);
  const savedInputRef = useRef<string>("");
  const wasOpenRef = useRef(false);
  const guidedListenerRef = useRef<boolean>(false);

  useEffect(() => {
    if (open) {
      setTimeout(() => inputRef.current?.focus(), 50);
      // Clear output when opening fresh (was not previously open)
      if (!wasOpenRef.current) {
        clear();
        setLastCommand(null);
        setGuidedMode(null);
      }
    }
    wasOpenRef.current = open;
  }, [open, clear]);

  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight;
    }
  }, [lines]);

  // When in guided "picking" step, watch CLI output for function list results
  useEffect(() => {
    if (!guidedListenerRef.current) return;
    if (isRunning) return;

    // CLI finished — parse the output
    guidedListenerRef.current = false;

    const fullOutput = lines
      .filter((l) => l.type === "chunk")
      .map((l) => (typeof l.value === "string" ? l.value : JSON.stringify(l.value)))
      .join("");

    const parsed = tryParseJson(fullOutput);
    if (parsed && Array.isArray(parsed)) {
      const fns = parsed.map((item: Record<string, unknown>) => ({
        id: (item.id as string) ?? (item.name as string) ?? String(item),
        type: (item.type as string) ?? undefined,
      }));
      setGuidedMode((prev) => (prev ? { ...prev, functions: fns, error: undefined } : prev));
    } else {
      // Check for errors
      const errorLine = lines.find((l) => l.type === "error");
      const errorMsg = errorLine
        ? typeof errorLine.value === "string"
          ? errorLine.value
          : JSON.stringify(errorLine.value)
        : "Failed to parse function list";
      setGuidedMode((prev) => (prev ? { ...prev, error: errorMsg } : prev));
    }
    clear();
  }, [lines, isRunning, clear]);

  const enterGuidedMode = useCallback(() => {
    setGuidedMode({ step: "picking", inputJson: "" });
    clear();
    guidedListenerRef.current = true;
    run(["functions", "list"]);
  }, [run, clear]);

  const guidedSelectFunction = useCallback((fnId: string) => {
    if (!fnId) {
      // Empty string means go back to picking step
      setGuidedMode((prev) => (prev ? { ...prev, step: "picking", selectedFunction: undefined } : prev));
      return;
    }
    setGuidedMode((prev) => (prev ? { ...prev, step: "input", selectedFunction: fnId } : prev));
  }, []);

  const guidedRun = useCallback(() => {
    if (!guidedMode || !guidedMode.selectedFunction) return;

    const args = ["functions", "executions", "create", "--function", guidedMode.selectedFunction];
    const trimmedInput = guidedMode.inputJson.trim();
    if (trimmedInput) {
      args.push("--input", trimmedInput);
    }

    const argsStr = `functions executions create --function ${guidedMode.selectedFunction}${trimmedInput ? ` --input '${trimmedInput}'` : ""}`;
    setLastCommand(argsStr);
    setInput(argsStr);

    // Add to history
    if (commandHistory[commandHistory.length - 1] !== argsStr) {
      commandHistory.push(argsStr);
      if (commandHistory.length > MAX_HISTORY) {
        commandHistory.shift();
      }
    }

    setGuidedMode(null);
    run(args);
  }, [guidedMode, run]);

  const exitGuidedMode = useCallback(() => {
    setGuidedMode(null);
    clear();
  }, [clear]);

  const submit = useCallback(() => {
    const trimmed = input.trim();
    if (!trimmed || isRunning) return;

    // Add to history
    if (commandHistory[commandHistory.length - 1] !== trimmed) {
      commandHistory.push(trimmed);
      if (commandHistory.length > MAX_HISTORY) {
        commandHistory.shift();
      }
    }
    historyIndexRef.current = -1;

    setLastCommand(trimmed);
    const args = trimmed.split(/\s+/);
    run(args);
  }, [input, isRunning, run]);

  const handleQuickAction = (argsStr: string, guided?: boolean) => {
    if (guided) {
      enterGuidedMode();
      return;
    }

    setInput(argsStr);
    setLastCommand(argsStr);

    // Add to history
    if (commandHistory[commandHistory.length - 1] !== argsStr) {
      commandHistory.push(argsStr);
      if (commandHistory.length > MAX_HISTORY) {
        commandHistory.shift();
      }
    }
    historyIndexRef.current = -1;

    run(argsStr.split(/\s+/));
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      submit();
      return;
    }

    if (e.key === "Escape") {
      onOpenChange(false);
      return;
    }

    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (commandHistory.length === 0) return;

      if (historyIndexRef.current === -1) {
        // Save current input before navigating history
        savedInputRef.current = input;
        historyIndexRef.current = commandHistory.length - 1;
      } else if (historyIndexRef.current > 0) {
        historyIndexRef.current -= 1;
      }
      setInput(commandHistory[historyIndexRef.current]);
      return;
    }

    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (historyIndexRef.current === -1) return;

      if (historyIndexRef.current < commandHistory.length - 1) {
        historyIndexRef.current += 1;
        setInput(commandHistory[historyIndexRef.current]);
      } else {
        // Back to the saved input
        historyIndexRef.current = -1;
        setInput(savedInputRef.current);
      }
      return;
    }
  };

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/60 z-50" />
        <Dialog.Content
          className="fixed top-[15%] left-1/2 -translate-x-1/2 w-[90vw] max-w-lg z-50 bg-ground-raised border border-node-border rounded-lg shadow-2xl overflow-hidden"
          onEscapeKeyDown={() => onOpenChange(false)}
        >
          <div className="p-4 border-b border-node-border">
            <Dialog.Title className="text-xs font-mono text-info-dim uppercase tracking-wide mb-3">
              Command Palette
            </Dialog.Title>
            <div className="flex gap-1.5 mb-3 flex-wrap">
              {QUICK_ACTIONS.map((action) => (
                <button
                  key={action.label}
                  onClick={() => handleQuickAction(action.args, action.guided)}
                  disabled={isRunning}
                  className="px-2 py-1 rounded-sm text-[10px] font-mono bg-ground-surface text-info-mid hover:text-copper-bright hover:bg-copper-warm/10 transition-colors disabled:opacity-50"
                >
                  {action.label}
                </button>
              ))}
            </div>
            {guidedMode ? (
              <GuidedExecution
                mode={guidedMode}
                isRunning={isRunning}
                onSelect={guidedSelectFunction}
                onUpdateInput={(json) => setGuidedMode((prev) => prev ? { ...prev, inputJson: json } : prev)}
                onRun={guidedRun}
                onBack={exitGuidedMode}
              />
            ) : (
              <>
                <div className="flex gap-2">
                  <input
                    ref={inputRef}
                    type="text"
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={handleKeyDown}
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
              </>
            )}
          </div>

          {!guidedMode && lines.length > 0 && (
            <div
              ref={outputRef}
              className="max-h-64 overflow-y-auto p-3 font-mono text-[11px] leading-relaxed bg-ground-base"
            >
              {lastCommand && (
                <div className="text-[10px] text-copper-dim mb-2 pb-1 border-b border-node-border/50">
                  $ objectiveai {lastCommand}
                </div>
              )}
              {lines.map((line, i) => (
                <OutputLine key={i} line={line} />
              ))}
              {isRunning && (
                <span className="inline-block w-1.5 h-3 bg-copper-bright animate-pulse" />
              )}
            </div>
          )}

          {!guidedMode && lines.length > 0 && !isRunning && (
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
