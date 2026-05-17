import * as Dialog from "@radix-ui/react-dialog";
import type { StoredSession } from "../../lib/storage";

function formatDate(ts: number): string {
  return new Date(ts).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatDuration(start: number, end: number | null): string {
  const ms = (end ?? Date.now()) - start;
  const seconds = Math.floor(ms / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
}

const KIND_STYLE: Record<string, string> = {
  "agent-completion": "text-copper-bright bg-copper-warm/20",
  execution: "text-info-mid bg-ground-surface",
  invention: "text-copper-mid bg-copper-mid/15",
  laboratory: "text-info-bright bg-ground-surface",
};

const KIND_SHORT: Record<string, string> = {
  "agent-completion": "Agent",
  execution: "Exec",
  invention: "Inv",
  laboratory: "Lab",
};

export function SessionPicker({
  open,
  onOpenChange,
  sessions,
  currentSessionId,
  onLoad,
  onDelete,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sessions: StoredSession[];
  currentSessionId: string;
  onLoad: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/60 z-40" />
        <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 w-full max-w-md max-h-[70vh] bg-ground-raised border border-node-border rounded-md shadow-xl flex flex-col">
          <div className="flex items-center justify-between px-4 py-3 border-b border-node-border">
            <Dialog.Title className="text-sm font-semibold text-info-bright">
              Sessions
            </Dialog.Title>
            <Dialog.Close className="p-1 rounded-sm text-info-dim hover:text-info-bright transition-colors">
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                <path d="M2 2l8 8M10 2l-8 8" />
              </svg>
            </Dialog.Close>
          </div>

          <div className="overflow-y-auto flex-1 p-2">
            {sessions.length === 0 && (
              <div className="text-center text-info-dim italic py-8 text-xs">
                No saved sessions
              </div>
            )}
            {sessions.map((session) => {
              const isCurrent = session.id === currentSessionId;
              return (
                <div
                  key={session.id}
                  className={`flex items-center gap-2 px-3 py-2.5 rounded-sm mb-1 text-xs ${
                    isCurrent
                      ? "bg-copper-warm/10 border border-copper-warm/30"
                      : "hover:bg-ground-surface"
                  }`}
                >
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-info-bright font-medium">
                        {formatDate(session.startTime)}
                      </span>
                      <span className="text-info-dim">
                        {formatDuration(session.startTime, session.endTime)}
                      </span>
                      {isCurrent && (
                        <span className="px-1.5 py-px rounded-sm bg-success/20 text-success text-[10px] font-mono">
                          Live
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-1.5 mt-1">
                      <span className="text-info-dim">{session.entryCount} entries</span>
                      {session.kinds.map((kind) => (
                        <span
                          key={kind}
                          className={`px-1 py-px rounded-sm font-mono text-[9px] ${KIND_STYLE[kind] ?? "text-info-dim bg-ground-surface"}`}
                        >
                          {KIND_SHORT[kind] ?? kind}
                        </span>
                      ))}
                    </div>
                  </div>
                  <div className="flex gap-1 shrink-0">
                    {!isCurrent && (
                      <>
                        <button
                          onClick={() => { onLoad(session.id); onOpenChange(false); }}
                          className="px-2 py-1 rounded-sm text-copper-bright hover:bg-copper-warm/20 transition-colors"
                        >
                          Load
                        </button>
                        <button
                          onClick={() => onDelete(session.id)}
                          className="px-2 py-1 rounded-sm text-error/70 hover:text-error hover:bg-error/10 transition-colors"
                        >
                          Delete
                        </button>
                      </>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
