import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

type RGBA = [number, number, number, number];

const css = readFileSync(new URL("./theme.css", import.meta.url), "utf8");
const appCss = readFileSync(new URL("./app.css", import.meta.url), "utf8");

// The five colours a theme is; everything else is mixed from them in theme.css.
const FIVE = ["--ground", "--raised", "--accent", "--ink", "--here"];

/** Each `selector { … }` block of theme.css, comments left out, with its custom properties in order. */
const blocks = [...css.replace(/\/\*[\s\S]*?\*\//g, "").matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((m) => ({
  selector: m[1].trim(),
  props: [...m[2].matchAll(/(--[\w-]+):\s*([^;]+);/g)].map((p) => [p[1], p[2].trim()] as [string, string]),
}));
const base = new Map(blocks.filter((b) => b.selector === ":root").flatMap((b) => b.props));

// Every theme in theme.css: plain :root is the default, and each :root[data-theme="<id>"] block swaps in its own
// five. A theme added there is checked here; there is no list to forget to add it to.
const THEMES: Record<string, Map<string, string>> = { "the default (:root)": new Map(base) };
for (const b of blocks) {
  const id = b.selector.match(/^:root\[data-theme="([\w-]+)"\]$/)?.[1];
  if (id) THEMES[id] = new Map([...base, ...b.props]);
}

describe("theme.css", () => {
  it("has the default and at least one more theme", () => {
    expect(Object.keys(THEMES).length).toBeGreaterThanOrEqual(2);
  });
  it("puts colours only in :root and in theme blocks", () => {
    expect(blocks.filter((b) => b.props.length).map((b) => b.selector).filter((s) => s !== ":root" && !/^:root\[data-theme="[\w-]+"\]$/.test(s))).toEqual([]);
  });
  it.each(blocks.filter((b) => b.selector.startsWith(":root[")).map((b) => [b.selector, b.props.map(([k]) => k)] as const))(
    "%s sets exactly the five colours",
    (_s, names) => {
      expect([...names].sort()).toEqual([...FIVE].sort());
    },
  );
  it("the default sets each of the five as a colour, and nothing else is a bare colour", () => {
    const hex = [...base].filter(([, v]) => /^#[0-9a-f]{6}$/i.test(v)).map(([k]) => k);
    // ok and bad are the only invented values (see theme.css); every other colour is one of the five or mixed from them.
    expect(hex.sort()).toEqual([...FIVE, "--ok", "--bad"].sort());
  });
});

/** A theme value, worked out: hex, `transparent`, `var(...)`, and `color-mix(in srgb, …)` (premultiplied, as CSS mixes). */
function colour(raw: Map<string, string>, v: string): RGBA | null {
  v = v.trim();
  if (/^#[0-9a-f]{6}$/i.test(v)) return [parseInt(v.slice(1, 3), 16), parseInt(v.slice(3, 5), 16), parseInt(v.slice(5, 7), 16), 1];
  if (v === "transparent") return [0, 0, 0, 0];
  const ref = v.match(/^var\((--[\w-]+)\)$/);
  if (ref) return token(raw, ref[1]);
  const mix = v.match(/^color-mix\(in srgb,\s*(.+?)\s+(\d+)%,\s*(.+)\)$/);
  if (mix) {
    const a = colour(raw, mix[1]);
    const b = colour(raw, mix[3]);
    const p = Number(mix[2]) / 100;
    if (!a || !b) return null;
    const alpha = a[3] * p + b[3] * (1 - p);
    if (alpha === 0) return [0, 0, 0, 0];
    const ch = (i: number) => (a[i] * a[3] * p + b[i] * b[3] * (1 - p)) / alpha;
    return [ch(0), ch(1), ch(2), alpha];
  }
  return null;
}
function token(raw: Map<string, string>, name: string): RGBA | null {
  const v = raw.get(name);
  return v === undefined ? null : colour(raw, v);
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
function ratio(raw: Map<string, string>, fg: string, bg: string): number {
  const f = token(raw, fg);
  const b = token(raw, bg);
  if (!f || !b) throw new Error(`no colour for ${f ? bg : fg}`);
  return contrast(over(f, b), b);
}

// Every colour of words app.css puts on a background, by the tokens it names, and where:
// at rest and in the hover, picked and error states.
const WORDS: [string, string, string][] = [
  ["--text", "--bg", "the page"],
  ["--text", "--surface", "cards, sections, the side columns"],
  ["--text", "--surface-2", "the row you're on; code in a quiet banner"],
  ["--text", "--surface-3", "a hovered row, tab, chip or quiet button"],
  ["--text", "--note", "warning banners and cards, the knock card"],
  ["--text", "--bad-soft", "a rejected settings check"],
  ["--text", "--accent-soft", "the agent kind you picked, the filter you picked"],
  ["--text-2", "--bg", "quieter words on the page"],
  ["--text-2", "--surface", "quieter words on a card"],
  ["--text-2", "--surface-2", "chips, quiet cards"],
  ["--text-2", "--surface-3", "a step number; a hovered row's place name"],
  ["--text-2", "--note", "a knock's note"],
  ["--text-2", "--accent-soft", "the picked kind's blurb"],
  ["--text-3", "--bg", "muted words on the page"],
  ["--text-3", "--surface", "muted words on a card"],
  ["--text-3", "--surface-2", "muted words on the row you're on"],
  ["--text-3", "--surface-3", "muted words on a hovered row"],
  ["--text-3", "--note", "muted words on the knock card"],
  ["--accent", "--bg", "links and counts on the page"],
  ["--accent", "--surface", "links and counts on a card"],
  ["--accent", "--surface-3", "a count on a hovered rail row"],
  ["--accent", "--note", "a link on the knock card"],
  ["--accent", "--accent-soft", "an accent chip"],
  ["--on-accent", "--accent", "the main button"],
  ["--on-accent", "--accent-hover", "the main button under the mouse"],
  ["--on-bad", "--bad", "a danger button"],
  ["--on-bad", "--bad-hover", "a danger button under the mouse"],
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

// The edges of things you can press or type into, the bars that mark where you are, and the status
// dots, each against what's around it.
const EDGES: [string, string, string][] = [
  ["--line-strong", "--bg", "buttons, fields and agent kinds on the page"],
  ["--line-strong", "--surface", "buttons, fields and chips on a card"],
  ["--line-strong", "--surface-2", "the picked option in a segmented control"],
  ["--line-hover", "--surface-3", "the edge of anything under the mouse"],
  ["--on-accent", "--accent-hover", "the edge of the main button under the mouse (its words' colour: the ink would vanish on pink)"],
  ["--on-bad", "--bad-hover", "the edge of a danger button under the mouse"],
  ["--here", "--bg", "the bar under the open tab"],
  ["--here", "--surface", "the bar beside the rail row you're on (the rail scrolls over the ground)"],
  ["--here", "--surface-2", "the bar beside the row you're on"],
  ["--accent", "--bg", "the main button, a field being typed in"],
  ["--accent", "--surface", "the main button, the picked kind, the picked filter, a card that waits on you"],
  ["--accent", "--note", "the main button on the knock card"],
  ["--text-3", "--bg", "an idle or never-run agent's dot in the rail"],
  ["--text-3", "--surface", "an idle or never-run agent's dot in the inbox"],
  ["--text-3", "--surface-2", "that dot on the thread you're on"],
  ["--text-3", "--surface-3", "that dot on a hovered row"],
  ["--ok", "--surface-3", "a working agent's dot on a hovered row"],
  ["--bad", "--surface", "a field that needs fixing, a danger button, a stopped agent's mark"],
  ["--bad", "--surface-3", "a stopped agent's mark on a hovered row"],
  ["--bad-hover", "--surface", "a danger button under the mouse"],
];

// Everywhere something you can focus sits, so everywhere the ring is drawn.
const RING_ON = ["--bg", "--surface", "--surface-2", "--surface-3", "--note", "--accent-soft", "--ok-soft", "--bad-soft"];

describe.each(Object.entries(THEMES))("the theme: %s", (_name, raw) => {
  const rows = [
    ...WORDS.map(([fg, bg, where]) => ({ name: `${fg} on ${bg}`, where, min: 7, got: ratio(raw, fg, bg) })),
    ...EDGES.map(([edge, bg, where]) => ({ name: `${edge} edge on ${bg}`, where, min: 3, got: ratio(raw, edge, bg) })),
  ];

  it("has a colour for every token the pairs name", () => {
    for (const [a, b] of [...WORDS, ...EDGES]) {
      expect(token(raw, a), a).not.toBeNull();
      expect(token(raw, b), b).not.toBeNull();
    }
  });

  it.each(rows.map((r) => [r.name, r.min, r.where, r.got] as const))("%s reads at %d:1 or better (%s)", (_name, min, _where, got) => {
    expect(got).toBeGreaterThanOrEqual(min);
  });

  it("the focus ring is at least 2px and reads at 3:1 on everything it's drawn on", () => {
    const m = raw.get("--focus")?.match(/^0 0 0 (\d+(?:\.\d+)?)px (.+)$/);
    expect(m, "--focus is a ring: 0 0 0 <width> <colour>").toBeTruthy();
    expect(Number(m![1])).toBeGreaterThanOrEqual(2);
    const ring = colour(raw, m![2]);
    expect(ring).not.toBeNull();
    for (const bg of RING_ON) {
      const b = token(raw, bg)!;
      expect(contrast(over(ring!, b), b), `the ring on ${bg}`).toBeGreaterThanOrEqual(3);
    }
  });
});

describe("where you are", () => {
  it("is only ever a bar: app.css never colours words with --here", () => {
    const asWords = [...appCss.matchAll(/^.*color:\s*var\(--here\).*$/gm)].map((m) => m[0].trim());
    expect(asWords).toEqual([]);
  });
  it("every bar and every card edge is --mark wide", () => {
    const fixed = [...appCss.matchAll(/^.*border-left:\s*[3-9]px.*$/gm)].map((m) => m[0].trim());
    expect(fixed).toEqual([]);
  });
});
