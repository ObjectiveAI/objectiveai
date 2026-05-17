import { useState, useRef, useEffect } from "react";
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

function InlineName({
  session,
  onRename,
}: {
  session: StoredSession;
  onRename: (id: string, name: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editing]);

  const startEditing = () => {
    setValue(session.name || formatDate(session.startTime));
    setEditing(true);
  };

  const commit = () => {
    setEditing(false);
    const trimmed = value.trim();
    if (trimmed && trimmed !== formatDate(session.startTime)) {
      onRename(session.id, trimmed);
    }
  };

  const cancel = () => {
    setEditing(false);
  };

  if (editing) {
    return (
      <input
        ref={inputRef}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") cancel();
        }}
        className="bg-ground-surface border border-copper-dim rounded-sm px-1 py-px text-xs text-info-bright font-medium outline-none w-full max-w-[180px]"
      />
    );
  }

  if (session.name) {
    return (
      <div>
        <span
          onClick={startEditing}
          className="text-info-bright font-semibold cursor-pointer hover:underline"
        >
          {session.name}
        </span>
        <div className="text-info-dim text-[10px] mt-0.5">
          {formatDate(session.startTime)}
        </div>
      </div>
    );
  }

  return (
    <span
      onClick={startEditing}
      className="text-info-bright font-medium cursor-pointer hover:underline"
    >
      {formatDate(session.startTime)}
    </span>
  );
}

export function SessionPicker({
  open,
  onOpenChange,
  sessions,
  currentSessionId,
  onLoad,
  onDelete,
  onRename,
  onExport,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sessions: StoredSession[];
  currentSessionId: string;
  onLoad: (id: string) => void;
  onDelete: (id: string) => void;
  onRename: (id: string, name: string) => void;
  onExport: (id: string) => void;
}) {
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/60 z-50" />
        <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-[51] w-full max-w-md max-h-[70vh] bg-ground-raised border border-node-border rounded-md shadow-xl flex flex-col">
          <div className="flex items-center justify-between px-4 py-3 border-b border-node-border">
            <Dialog.Title className="text-sm font-semibold text-info-bright">
              Sessions
            </Dialog.Title>
            <Dialog.Close className="p-1 rounded-sm text-info-dim hover:text-info-bright hover:bg-ground-surface transition-colors" aria-label="Close sessions dialog">
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                <path d="M2 2l8 8M10 2l-8 8" />
              </svg>
            </Dialog.Close>
          </div>

          <div className="overflow-y-auto flex-1 p-2">
            {sessions.length === 0 && (
              <div className="text-center py-8">
                <svg className="w-6 h-6 mx-auto mb-3 text-info-dim/20" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round">
                  <rect x="3" y="4" width="14" height="12" rx="2" />
                  <path d="M7 4V2M13 4V2M3 8h14" />
                </svg>
                <p className="text-info-dim text-xs">No saved sessions</p>
                <p className="text-info-dim/50 text-[10px] mt-1">Sessions are saved automatically when you run commands</p>
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
                      <InlineName session={session} onRename={onRename} />
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
                    {isCurrent ? (
                      <button
                        onClick={() => onExport(session.id)}
                        className="px-2 py-1 rounded-sm text-info-dim hover:text-info-bright hover:bg-ground-surface transition-colors"
                      >
                        Export
                      </button>
                    ) : (
                      <>
                        <button
                          onClick={() => { onLoad(session.id); onOpenChange(false); }}
                          className="px-2 py-1 rounded-sm text-copper-bright hover:bg-copper-warm/20 transition-colors"
                        >
                          Load
                        </button>
                        <button
                          onClick={() => onExport(session.id)}
                          className="px-2 py-1 rounded-sm text-info-dim hover:text-info-bright hover:bg-ground-surface transition-colors"
                        >
                          Export
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
