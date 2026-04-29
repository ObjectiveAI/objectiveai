export function scoreColor(s: number): string {
  if (s >= 0.5) return "var(--copper-hot)";
  if (s >= 0.3) return "var(--copper-mid)";
  if (s >= 0.15) return "var(--copper-warm)";
  return "var(--copper-dim)";
}

export function pct(n: number): string {
  return (n * 100).toFixed(1) + "%";
}

export function dotPct(n: number): string {
  return "." + (n * 100).toFixed(0).padStart(2, "0");
}

export function stateColor(s: string): string {
  if (s === "complete") return "var(--copper-hot)";
  if (s === "streaming") return "var(--copper-mid)";
  if (s === "error") return "var(--error)";
  return "var(--node-border)";
}

export function labelFor(r: unknown, i: number): string {
  if (typeof r === "string") return r.length > 24 ? r.slice(0, 22) + "…" : r;
  if (r && typeof r === "object") {
    const o = r as Record<string, unknown>;
    if (typeof o.text === "string") return o.text.length > 24 ? o.text.slice(0, 22) + "…" : o.text;
  }
  return `#${i + 1}`;
}
