import { memo, useMemo, useRef, useEffect, useState } from "react";
import * as Collapsible from "@radix-ui/react-collapsible";
import { AgentCompletionView } from "../../AgentCompletionView";
import { FunctionInventionRecursiveView } from "../../FunctionInventionRecursiveView";
import { FunctionExecutionView } from "../../FunctionExecutionView";
import { LaboratoryExecutionView } from "./LaboratoryExecutionView";
import { ErrorBoundary } from "../shared/ErrorBoundary";
import { InnerErrorsBadge } from "../shared/InnerErrorsBadge";
import { getEntrySummary } from "../../lib/entrySummary";
import { countInnerErrors } from "../../lib/innerErrors";
import type { Entry } from "../../types";

function EntryContent({ entry }: { entry: Entry }) {
  if (entry.kind === "agent-completion") {
    return <AgentCompletionView entry={entry} />;
  }
  if (entry.kind === "invention") {
    return <FunctionInventionRecursiveView entry={entry} />;
  }
  if (entry.kind === "execution") {
    return <FunctionExecutionView entry={entry} />;
  }
  if (entry.kind === "laboratory") {
    return <LaboratoryExecutionView entry={entry} />;
  }
  return null;
}

const STATUS_COLOR = {
  streaming: "bg-copper-hot animate-pulse",
  complete: "bg-success",
  error: "bg-error",
};

const KIND_ACCENT: Record<string, string> = {
  "agent-completion": "border-l-kind-agent",
  execution: "border-l-kind-execution",
  invention: "border-l-kind-invention",
  laboratory: "border-l-kind-laboratory",
};

const KIND_LABEL_STYLE: Record<string, string> = {
  Agent: "text-kind-agent bg-kind-agent/15",
  Execution: "text-kind-execution bg-kind-execution/15",
  Invention: "text-kind-invention bg-kind-invention/15",
  Laboratory: "text-kind-laboratory bg-kind-laboratory/15",
};

function entryTimestamp(entry: Entry): string {
  const c = entry.chunk && "created" in entry.chunk
    ? (entry.chunk as { created?: number }).created
    : undefined;
  if (c == null) return "";
  return new Date(c * 1000).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

export const EntryView = memo(function EntryView({
  entry,
  collapsed = false,
  onToggle,
  selected = false,
}: {
  entry: Entry;
  collapsed?: boolean;
  onToggle?: () => void;
  selected?: boolean;
}) {
  const summary = getEntrySummary(entry);
  const innerErrorCount = useMemo(() => countInnerErrors(entry), [entry.chunk]);
  const timeStr = entryTimestamp(entry);

  const [isNew, setIsNew] = useState(true);
  const mountRef = useRef(true);
  useEffect(() => {
    if (mountRef.current) {
      mountRef.current = false;
      const t = requestAnimationFrame(() => setIsNew(false));
      return () => cancelAnimationFrame(t);
    }
  }, []);

  const accentClass = KIND_ACCENT[entry.kind] ?? "";

  return (
    <Collapsible.Root open={!collapsed} onOpenChange={onToggle}>
      <Collapsible.Trigger data-entry-trigger className={`group flex items-center gap-2 max-w-content mx-auto w-full pl-3 pr-4 h-10 cursor-pointer rounded-md border border-l-2 ${accentClass} bg-ground-raised text-xs select-none focus:border-copper-dim transition-all duration-300 mb-1 entry-row ${isNew ? "opacity-0 translate-y-1" : "opacity-100 translate-y-0"} ${selected ? "border-copper-warm/50 bg-ground-surface" : "border-node-border"}`}>
        <svg
          className="w-2 h-2 text-info-dim transition-transform duration-150 group-data-[state=open]:rotate-90 shrink-0"
          viewBox="0 0 8 8"
          fill="currentColor"
        >
          <path d="M2 1l4 3-4 3z" />
        </svg>
        <div className={`w-1.5 h-1.5 rounded-full shrink-0 ${STATUS_COLOR[summary.status]}`} />
        <span className={`px-1.5 py-px rounded-sm font-mono text-[10px] shrink-0 ${KIND_LABEL_STYLE[summary.kindLabel] ?? "text-info-dim bg-ground-surface"}`}>
          {summary.kindLabel}
        </span>
        <span className="text-info-bright font-semibold truncate">{summary.title}</span>
        {summary.detail && <span className="text-info-dim truncate hidden sm:inline">{summary.detail}</span>}
        <InnerErrorsBadge count={innerErrorCount} />
        <div className="ml-auto flex items-center gap-2 shrink-0">
          {summary.tokens > 0 && <span className="font-mono text-[10px] text-info-dim/70 tabular-nums hidden sm:inline">{summary.tokens.toLocaleString()}t</span>}
          {summary.messageCount > 0 && <span className="font-mono text-[10px] text-info-dim/50 tabular-nums hidden sm:inline">{summary.messageCount}msg</span>}
          {timeStr && <span className="font-mono text-[10px] text-info-dim/50">{timeStr}</span>}
          <span className="font-mono text-[10px] text-info-dim/40">{entry.id.slice(0, 8)}</span>
        </div>
      </Collapsible.Trigger>

      <Collapsible.Content className="overflow-hidden">
        <ErrorBoundary>
          <EntryContent entry={entry} />
        </ErrorBoundary>
      </Collapsible.Content>
    </Collapsible.Root>
  );
});
