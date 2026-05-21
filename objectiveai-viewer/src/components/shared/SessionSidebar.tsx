import cn from "classnames";
import type { StoredSession } from "../../lib/storage";

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

function formatDate(ts: number): string {
  return new Date(ts).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

const KIND_DOT: Record<string, string> = {
  "agent-completion": "bg-kind-agent",
  execution: "bg-kind-execution",
  invention: "bg-kind-invention",
  laboratory: "bg-kind-laboratory",
};

export function SessionSidebar({
  sessions,
  currentSessionId,
  onLoad,
}: {
  sessions: StoredSession[];
  currentSessionId: string;
  onLoad: (id: string) => void;
}) {
  return (
    <div className={cn("py-2", "px-1.5")}>
      <div className={cn("px-2", "py-1.5", "text-[9px]", "font-mono", "uppercase", "tracking-widest", "text-info-dim", "select-none")}>
        Sessions
      </div>
      {sessions.length === 0 && (
        <div className={cn("px-2", "py-8", "text-center")}>
          <svg className={cn("w-5", "h-5", "mx-auto", "mb-2", "text-info-dim/30")} viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round">
            <rect x="3" y="4" width="14" height="12" rx="2" />
            <path d="M7 4V2M13 4V2M3 8h14" />
          </svg>
          <p className={cn("text-[10px]", "text-info-dim/50", "leading-relaxed")}>Sessions appear here<br />as you use the viewer</p>
        </div>
      )}
      {sessions.map((session) => {
        const isCurrent = session.id === currentSessionId;
        return (
          <button
            key={session.id}
            onClick={() => { if (!isCurrent) onLoad(session.id); }}
            aria-current={isCurrent ? "true" : undefined}
            className={cn(
              "w-full",
              "text-left",
              "px-2",
              "py-1.5",
              "rounded-sm",
              "mb-px",
              "text-[10px]",
              "transition-colors",
              "border",
              isCurrent
                ? cn("bg-copper-warm/10", "border-copper-warm/30")
                : cn("hover:bg-ground-surface", "border-transparent"),
            )}
          >
            <div className={cn("flex", "items-center", "gap-1.5")}>
              {isCurrent && <span className={cn("w-1.5", "h-1.5", "rounded-full", "bg-success", "animate-pulse", "shrink-0")} />}
              <span className={cn("text-info-bright", "font-medium", "truncate")}>
                {session.name || formatDate(session.startTime)}
              </span>
              <span className={cn("ml-auto", "text-info-dim", "shrink-0")}>
                {formatTime(session.startTime)}
              </span>
            </div>
            <div className={cn("flex", "items-center", "gap-1", "mt-0.5", "pl-3")}>
              <span className={cn("text-info-mid")}>{session.entryCount}</span>
              {session.kinds.map((kind) => (
                <span key={kind} className={cn("w-1.5", "h-1.5", "rounded-full", KIND_DOT[kind] ?? "bg-info-dim")} />
              ))}
            </div>
          </button>
        );
      })}
    </div>
  );
}
