export function scoreColor(s: number): string {
  if (s >= 0.5) return "var(--color-copper-hot)";
  if (s >= 0.3) return "var(--color-copper-mid)";
  if (s >= 0.15) return "var(--color-copper-warm)";
  return "var(--color-copper-dim)";
}

export function formatScore(s: number): string {
  return s.toFixed(3);
}
