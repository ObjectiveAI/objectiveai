function timeAgo(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ago`;
}

export function RestoreBanner({
  entryCount,
  timestamp,
  onDismiss,
  onBrowse,
}: {
  entryCount: number;
  timestamp: number;
  onDismiss: () => void;
  onBrowse: () => void;
}) {
  return (
    <div className="flex items-center gap-3 px-4 py-2 border-b border-copper-warm/30 bg-copper-warm/5 text-xs text-info-mid select-none">
      <span>
        Restored {entryCount} entries from {timeAgo(Date.now() - timestamp)}
      </span>
      <div className="ml-auto flex gap-2">
        <button
          onClick={onBrowse}
          className="px-2 py-0.5 rounded-sm text-copper-bright hover:bg-copper-warm/20 transition-colors"
        >
          Browse sessions
        </button>
        <button
          onClick={onDismiss}
          className="px-2 py-0.5 rounded-sm text-info-dim hover:text-info-mid transition-colors"
        >
          Dismiss
        </button>
      </div>
    </div>
  );
}
