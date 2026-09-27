import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

/** The theme's colours, worked out: hex values, `var(...)`, and `color-mix(in srgb, …)`. */
function tokens(): Map<string, [number, number, number]> {
  const css = readFileSync(new URL("./theme.css", import.meta.url), "utf8");
  const raw = new Map<string, string>();
  for (const m of css.matchAll(/(--[\w-]+):\s*([^;]+);/g)) raw.set(m[1], m[2].trim());
  const out = new Map<string, [number, number, number]>();
  const hex = (h: string): [number, number, number] => [parseInt(h.slice(1, 3), 16), parseInt(h.slice(3, 5), 16), parseInt(h.slice(5, 7), 16)];
  const value = (v: string): [number, number, number] | null => {
    v = v.trim();
    if (/^#[0-9a-f]{6}$/i.test(v)) return hex(v);
    const ref = v.match(/^var\((--[\w-]+)\)$/);
    if (ref) return resolve(ref[1]);
    const mix = v.match(/^color-mix\(in srgb,\s*(.+?)\s+(\d+)%,\s*(.+)\)$/);
    if (mix) {
      const a = value(mix[1]);
      const b = value(mix[3]);
      const p = Number(mix[2]) / 100;
      if (!a || !b) return null;
      return [0, 1, 2].map((i) => a[i] * p + b[i] * (1 - p)) as [number, number, number];
    }
    return null;
  };
  const resolve = (name: string): [number, number, number] | null => {
    if (out.has(name)) return out.get(name)!;
    const v = raw.get(name);
    const c = v ? value(v) : null;
    if (c) out.set(name, c);
    return c;
  };
  for (const name of raw.keys()) resolve(name);
  return out;
}

function luminance([r, g, b]: [number, number, number]): number {
  const ch = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b);
}

function contrast(a: [number, number, number], b: [number, number, number]): number {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

describe("the theme", () => {
  const t = tokens();
  // Every text colour the app puts on a background, as a pair of tokens.
  const pairs: [string, string][] = [
    ["--text", "--bg"],
    ["--text", "--surface"],
    ["--text", "--surface-2"],
    ["--text-2", "--bg"],
    ["--text-2", "--surface"],
    ["--text-3", "--bg"],
    ["--text-3", "--surface"],
    ["--on-accent", "--accent"],
    ["--plum", "--bad"],
    ["--plum", "--bad-hover"],
    ["--bad", "--bad-soft"],
    ["--ok", "--ok-soft"],
    ["--warn", "--warn-soft"],
    ["--here", "--bg"],
    ["--here", "--surface"],
    ["--accent", "--bg"],
  ];
  it.each(pairs)("%s on %s is AAA (7:1)", (fg, bg) => {
    const a = t.get(fg);
    const b = t.get(bg);
    expect(a, fg).toBeDefined();
    expect(b, bg).toBeDefined();
    expect(contrast(a!, b!)).toBeGreaterThanOrEqual(7);
  });
});
