export function InnerErrorsBadge({ count }: { count: number }) {
  if (count === 0) return null;
  return (
    <span className="px-1.5 py-px rounded-sm text-[10px] font-mono bg-amber-500/15 text-amber-400 shrink-0">
      {count} {count === 1 ? "error" : "errors"}
    </span>
  );
}
