export function scoreColor(s: number): string {
  if (s >= 0.5) return "var(--color-copper-hot)";
  if (s >= 0.3) return "var(--color-copper-mid)";
  if (s >= 0.15) return "var(--color-copper-warm)";
  return "var(--color-copper-dim)";
}

export function stateColor(s: string): string {
  if (s === "complete") return "var(--color-copper-hot)";
  if (s === "streaming") return "var(--color-copper-mid)";
  if (s === "error") return "var(--color-error)";
  return "var(--color-node-border)";
}

export function formatScore(s: number): string {
  return s.toFixed(3);
}
