import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

type RGBA = [number, number, number, number];

const css = readFileSync(new URL("./theme.css", import.meta.url), "utf8");
const raw = new Map<string, string>();
for (const m of css.matchAll(/(--[\w-]+):\s*([^;]+);/g)) raw.set(m[1], m[2].trim());

/** A theme value, worked out: hex, `transparent`, `var(...)`, and `color-mix(in srgb, …)` (premultiplied, as CSS mixes). */
function colour(v: string): RGBA | null {
  v = v.trim();
  if (/^#[0-9a-f]{6}$/i.test(v)) return [parseInt(v.slice(1, 3), 16), parseInt(v.slice(3, 5), 16), parseInt(v.slice(5, 7), 16), 1];
  if (v === "transparent") return [0, 0, 0, 0];
  const ref = v.match(/^var\((--[\w-]+)\)$/);
  if (ref) return token(ref[1]);
  const mix = v.match(/^color-mix\(in srgb,\s*(.+?)\s+(\d+)%,\s*(.+)\)$/);
  if (mix) {
    const a = colour(mix[1]);
    const b = colour(mix[3]);
    const p = Number(mix[2]) / 100;
    if (!a || !b) return null;
    const alpha = a[3] * p + b[3] * (1 - p);
    if (alpha === 0) return [0, 0, 0, 0];
    const ch = (i: number) => (a[i] * a[3] * p + b[i] * b[3] * (1 - p)) / alpha;
    return [ch(0), ch(1), ch(2), alpha];
  }
  return null;
}
function token(name: string): RGBA | null {
  const v = raw.get(name);
  return v === undefined ? null : colour(v);
}

/** `top` painted over an opaque `under`. */
const over = (top: RGBA, under: RGBA): RGBA => [0, 1, 2].map((i) => top[i] * top[3] + under[i] * (1 - top[3])).concat(1) as RGBA;

function luminance([r, g, b]: RGBA): number {
  const ch = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b);
}
function contrast(a: RGBA, b: RGBA): number {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}
/** How a colour reads painted on a background (both by token). */
function ratio(fg: string, bg: string): number {
  const f = token(fg);
  const b = token(bg);
  if (!f || !b) throw new Error(`no colour for ${f ? bg : fg}`);
  return contrast(over(f, b), b);
}

// Every colour of words app.css puts on a background, by the tokens it names, and where:
// at rest and in the hover, picked and error states.
const WORDS: [string, string, string][] = [
  ["--text", "--bg", "the page"],
  ["--text", "--surface", "cards, sections, the side columns"],
  ["--text", "--surface-2", "a hovered button or row; code in a quiet banner"],
  ["--text", "--surface-3", "a hovered quiet button or rail row"],
  ["--text", "--note", "warning banners and cards, the knock card"],
  ["--text", "--accent-soft", "the agent kind you picked"],
  ["--text-2", "--bg", "quieter words on the page"],
  ["--text-2", "--surface", "quieter words on a card"],
  ["--text-2", "--surface-2", "chips, quiet cards"],
  ["--text-2", "--surface-3", "a step number; a hovered row's place name"],
  ["--text-2", "--note", "a knock's note"],
  ["--text-2", "--accent-soft", "the picked kind's blurb"],
  ["--text-3", "--bg", "muted words on the page"],
  ["--text-3", "--surface", "muted words on a card"],
  ["--text-3", "--surface-2", "muted words on a picked or hovered row"],
  ["--text-3", "--surface-3", "a hovered rail row's kind"],
  ["--text-3", "--note", "muted words on the knock card"],
  ["--here", "--bg", "the open tab, the place you are"],
  ["--here", "--surface", "the rail row you're on"],
  ["--here", "--surface-2", "the picked row in a list"],
  ["--accent", "--bg", "links and counts on the page"],
  ["--accent", "--surface", "links and counts on a card"],
  ["--accent", "--surface-3", "a count on a hovered rail row"],
  ["--pink", "--note", "a link on the knock card"],
  ["--pink", "--accent-soft", "an accent chip"],
  ["--on-accent", "--accent", "the main button"],
  ["--on-accent", "--accent-hover", "the main button under the mouse"],
  ["--plum", "--bad", "a danger button"],
  ["--plum", "--bad-hover", "a danger button under the mouse"],
  ["--warn", "--bg", "a warning word"],
  ["--warn", "--warn-soft", "a warning chip"],
  ["--ok", "--bg", "how it went, on the page"],
  ["--ok", "--surface", "how it went, on a card"],
  ["--ok", "--note", "a vouch that holds, on the knock card"],
  ["--ok", "--ok-soft", "a done chip"],
  ["--bad", "--bg", "what failed, on the page"],
  ["--bad", "--surface", "a failed step"],
  ["--bad", "--bad-soft", "an error card or chip"],
];

// The edges of things you can press or type into, against what's around them.
const EDGES: [string, string, string][] = [
  ["--line-strong", "--bg", "buttons and fields on the page"],
  ["--line-strong", "--surface", "buttons and fields on a card"],
  ["--line", "--bg", "the open tab, a room chip"],
  ["--line", "--surface", "a room chip, an agent kind"],
  ["--line", "--surface-2", "the picked option in a segmented control"],
  ["--accent", "--bg", "the main button, a field being typed in"],
  ["--accent", "--surface", "the main button, the picked kind, a card that waits on you"],
  ["--accent", "--note", "the main button on the knock card"],
  ["--here", "--surface", "the Home view you're on"],
  ["--bad", "--surface", "a field that needs fixing, a danger button"],
  ["--bad-hover", "--surface", "a danger button under the mouse"],
];

// Everywhere something you can focus sits, so everywhere the ring is drawn.
const RING_ON = ["--bg", "--surface", "--surface-2", "--surface-3", "--note", "--accent-soft", "--ok-soft", "--bad-soft"];

// Today's breaks. Each waits on a colour pick (or, for --line, on which edge marks what can be
// pressed), which is not this test's to make. The list is checked both ways: every pair on it
// must still break, and every pair not on it must hold.
const BREAKS: Record<string, string> = {
  "--text-3 on --note": "muted words on the knock card: --text-3 or --note",
  "--text-3 on --surface-3": "a hovered rail row's kind: --text-3 or --surface-3",
  "--line-strong edge on --bg": "the edge that marks a press: --line-strong",
  "--line-strong edge on --surface": "the edge that marks a press: --line-strong",
  "--line edge on --bg": "pressables drawn with the quiet rule (the open tab, room chips)",
  "--line edge on --surface": "pressables drawn with the quiet rule (room chips, agent kinds)",
  "--line edge on --surface-2": "pressables drawn with the quiet rule (the picked option in a segmented control)",
};

const rows = [
  ...WORDS.map(([fg, bg, where]) => ({ name: `${fg} on ${bg}`, where, min: 7, got: ratio(fg, bg) })),
  ...EDGES.map(([edge, bg, where]) => ({ name: `${edge} edge on ${bg}`, where, min: 3, got: ratio(edge, bg) })),
];

describe("the theme", () => {
  it("has a colour for every token the pairs name", () => {
    for (const [a, b] of [...WORDS, ...EDGES]) {
      expect(token(a), a).not.toBeNull();
      expect(token(b), b).not.toBeNull();
    }
  });

  it.each(rows.filter((r) => !(r.name in BREAKS)).map((r) => [r.name, r.min, r.where, r.got] as const))("%s reads at %d:1 or better (%s)", (_name, min, _where, got) => {
    expect(got).toBeGreaterThanOrEqual(min);
  });

  it("the focus ring is at least 2px and reads at 3:1 on everything it's drawn on", () => {
    const m = raw.get("--focus")?.match(/^0 0 0 (\d+(?:\.\d+)?)px (.+)$/);
    expect(m, "--focus is a ring: 0 0 0 <width> <colour>").toBeTruthy();
    expect(Number(m![1])).toBeGreaterThanOrEqual(2);
    const ring = colour(m![2]);
    expect(ring).not.toBeNull();
    for (const bg of RING_ON) {
      const b = token(bg)!;
      expect(contrast(over(ring!, b), b), `the ring on ${bg}`).toBeGreaterThanOrEqual(3);
    }
  });

  it("today's breaks are exactly the named ones, and each still breaks", () => {
    const names = new Set(rows.map((r) => r.name));
    for (const name of Object.keys(BREAKS)) expect(names.has(name), `${name} is a pair the app draws`).toBe(true);
    const breaking = rows.filter((r) => r.got < r.min).map((r) => r.name).sort();
    expect(breaking).toEqual(Object.keys(BREAKS).sort());
  });
});
