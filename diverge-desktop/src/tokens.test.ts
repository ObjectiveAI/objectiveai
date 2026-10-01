import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import { describe, expect, it } from "vitest";

/** One declaration in a style sheet, with the selector and at-rules it sits under. */
type Decl = { at: string; line: number; selector: string; within: string[]; prop: string; value: string };

const SRC = fileURLToPath(new URL(".", import.meta.url));
const SHEETS = ["theme.css", "app.css"];

/** A small reader for the app's own sheets: comments blanked (lines kept), blocks nested by braces. */
function declarations(name: string): Decl[] {
  const text = readFileSync(join(SRC, name), "utf8").replace(/\/\*[\s\S]*?\*\//g, (c) => c.replace(/[^\n]/g, " "));
  const out: Decl[] = [];
  const stack: string[] = [];
  let buf = "";
  let from = 0;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === "{") {
      stack.push(buf.trim().replace(/\s+/g, " "));
      buf = "";
      from = i + 1;
    } else if (c === ";" || c === "}") {
      const d = buf.trim();
      const colon = d.indexOf(":");
      if (d && stack.length && colon > 0) {
        const lead = buf.length - buf.trimStart().length;
        out.push({
          at: name,
          line: text.slice(0, from + lead).split("\n").length,
          selector: stack[stack.length - 1],
          within: stack.slice(0, -1),
          prop: d.slice(0, colon).trim(),
          value: d.slice(colon + 1).trim(),
        });
      }
      buf = "";
      from = i + 1;
      if (c === "}") stack.pop();
    } else buf += c;
  }
  return out;
}

const decls = SHEETS.flatMap(declarations);
const where = (d: Decl) => `${d.at}:${d.line} ${d.selector} { ${d.prop}: ${d.value} }`;
const defined = new Set(decls.filter((d) => d.selector === ":root" && d.prop.startsWith("--")).map((d) => d.prop));
const rootValue = new Map(decls.filter((d) => d.selector === ":root" && d.prop.startsWith("--")).map((d) => [d.prop, d.value]));

/** A --text-* token on :root whose value is a length, or max()/min()/clamp() of lengths and other size tokens. */
function isSize(name: string): boolean {
  const v = rootValue.get(name);
  if (!name.startsWith("--text-") || v === undefined) return false;
  const length = (a: string) => /^\d*\.?\d+(px|rem|em)$/.test(a);
  if (length(v)) return true;
  const fn = v.match(/^(max|min|clamp)\((.*)\)$/);
  return !!fn && fn[2].split(/\s*,\s*/).every((a) => length(a) || (/^var\(--text-[\w-]+\)$/.test(a) && a.slice(4, -1) !== name && isSize(a.slice(4, -1))));
}

const parts = (value: string) => value.replace(/\s*!important$/, "").split(/\s+(?![^(]*\))/);

/** The app's own code under src/ (.ts and .tsx), tests left out. */
function code(): string[] {
  const files: string[] = [];
  const walk = (dir: string) => {
    for (const n of readdirSync(dir).sort()) {
      const p = join(dir, n);
      if (statSync(p).isDirectory()) walk(p);
      else if (/\.tsx?$/.test(n) && !/\.test\.tsx?$/.test(n)) files.push(p);
    }
  };
  walk(SRC);
  return files;
}
const tsx = () => code().filter((f) => f.endsWith(".tsx"));

describe("tokens", () => {
  it("spacing (padding, margin, gap, inset) comes only from --space-N, 0 or auto", () => {
    const ok = (p: string) => p === "0" || p === "auto" || (/^var\(--space-\d+\)$/.test(p) && defined.has(p.slice(4, -1)));
    const off = decls.filter((d) => /^(padding|margin|inset)(-[a-z-]+)?$|^(row-|column-)?gap$/.test(d.prop) && !parts(d.value).every(ok));
    expect(off.map(where)).toEqual([]);
    // …and none from a style object in the screens, where the sheet can't see it.
    const inline = tsx().flatMap((f) =>
      readFileSync(f, "utf8")
        .split("\n")
        .flatMap((l, i) => (/\b(padding|margin|inset)(Top|Right|Bottom|Left|Inline|Block)?\w*\s*:|\b(gap|rowGap|columnGap)\s*:/.test(l) ? [`${f.slice(SRC.length)}:${i + 1}`] : [])),
    );
    expect(inline).toEqual([]);
  });

  it("font sizes come only from the --text-* size tokens", () => {
    const off = decls.filter((d) => d.prop === "font-size" && !(/^var\(--text-[\w-]+\)$/.test(d.value) && isSize(d.value.slice(4, -1))));
    expect(off.map(where)).toEqual([]);
    // The --text-* colours are not sizes, so a font size can't borrow one.
    expect(["--text-xs", "--text-mono"].every(isSize)).toBe(true);
    expect(["--text", "--text-2", "--text-3"].some(isSize)).toBe(false);
  });

  it("app.css names no colour of its own: every colour is a theme role", () => {
    const named = /#[0-9a-f]{3,8}\b|\b(rgba?|hsla?|hwb|lab|lch|oklab|oklch|color|color-mix)\(|\b(white|black|red|green|blue|gray|grey|pink|gold|orange|purple|yellow)\b/i;
    const off = declarations("app.css").filter((d) => !d.prop.startsWith("--") && named.test(d.value.replace(/var\(--[\w-]+\)/g, "")));
    expect(off.map(where)).toEqual([]);
  });

  it("native controls take the theme's accent once, on :root", () => {
    const set = decls.filter((d) => d.prop === "accent-color");
    expect(set.map((d) => `${d.at} ${d.selector} ${d.value}`)).toEqual(["theme.css :root var(--accent)"]);
  });

  it("every endless animation stops when the system asks for less motion", () => {
    const calm = new Set(
      decls
        .filter((d) => d.within.some((w) => /^@media\b.*prefers-reduced-motion:\s*reduce/.test(w)) && /^animation(-name)?$/.test(d.prop) && d.value === "none")
        .flatMap((d) => d.selector.split(",").map((s) => s.trim())),
    );
    const endless = decls.filter((d) => /^animation(-iteration-count)?$/.test(d.prop) && /\binfinite\b/.test(d.value));
    expect(endless.length).toBeGreaterThan(0);
    expect(endless.filter((d) => !d.selector.split(",").every((s) => calm.has(s.trim()))).map(where)).toEqual([]);
  });

  it("every class app.css styles is named in the screens", () => {
    // Class words: the strings inside each className={…} in a .tsx file. A template there
    // like `chip-${tone}` names chip-<word> for any word the code spells as a string.
    const files = code();
    const named = new Set<string>();
    const prefixes = new Set<string>();
    const strings = new Set<string>();
    for (const f of files) {
      const source = ts.createSourceFile(f, readFileSync(f, "utf8"), ts.ScriptTarget.Latest, true, f.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
      const visit = (n: ts.Node, inClass: boolean) => {
        if (ts.isJsxAttribute(n) && n.name.getText(source) === "className") inClass = true;
        if (ts.isStringLiteral(n) || ts.isNoSubstitutionTemplateLiteral(n) || ts.isTemplateHead(n) || ts.isTemplateMiddle(n) || ts.isTemplateTail(n)) {
          const words = n.text.split(/\s+/).filter(Boolean);
          for (const w of words) strings.add(w);
          if (inClass && f.endsWith(".tsx")) {
            for (const w of words) named.add(w);
            if ((ts.isTemplateHead(n) || ts.isTemplateMiddle(n)) && /\S-$/.test(n.text)) prefixes.add(words[words.length - 1]);
          }
        }
        ts.forEachChild(n, (c) => visit(c, inClass));
      };
      visit(source, false);
    }
    const styled = new Set<string>();
    for (const d of declarations("app.css")) for (const m of d.selector.matchAll(/\.(-?[_a-zA-Z][\w-]*)/g)) styled.add(m[1]);
    const unnamed = [...styled].filter((c) => !named.has(c) && ![...prefixes].some((p) => c.startsWith(p) && strings.has(c.slice(p.length)))).sort();
    expect(unnamed).toEqual([]);
  });
});

describe("the stand-in label", () => {
  it("sits outside every part of the rail that scrolls, so it can't scroll out of view", () => {
    // Classes app.css makes scroll (from the last part of each selector that sets overflow to auto or scroll).
    const scrolls = new Set<string>();
    for (const d of declarations("app.css")) {
      if (!/^overflow(-y)?$/.test(d.prop) || !/\b(auto|scroll)\b/.test(d.value)) continue;
      for (const s of d.selector.split(",")) for (const m of (s.trim().split(/[\s>+~]+/).pop() ?? "").matchAll(/\.([\w-]+)/g)) scrolls.add(m[1]);
    }
    const file = join(SRC, "components", "Rail.tsx");
    const source = ts.createSourceFile(file, readFileSync(file, "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
    // The class words an element's className spells as plain text.
    const classes = (el: ts.JsxOpeningLikeElement) =>
      el.attributes.properties.flatMap((a) => {
        if (!ts.isJsxAttribute(a) || a.name.getText(source) !== "className" || !a.initializer) return [];
        const words: string[] = [];
        const visit = (n: ts.Node) => {
          if (ts.isStringLiteral(n) || ts.isNoSubstitutionTemplateLiteral(n) || ts.isTemplateHead(n) || ts.isTemplateMiddle(n) || ts.isTemplateTail(n)) words.push(...n.text.split(/\s+/).filter(Boolean));
          ts.forEachChild(n, visit);
        };
        visit(a.initializer);
        return words;
      });
    const opening = (n: ts.Node) => (ts.isJsxElement(n) ? n.openingElement : ts.isJsxSelfClosingElement(n) ? n : null);
    const all: ts.Node[] = [];
    const walk = (n: ts.Node) => {
      if (opening(n)) all.push(n);
      ts.forEachChild(n, walk);
    };
    walk(source);
    const label = all.filter((n) => classes(opening(n)!).includes("stand-in-line"));
    expect(label).toHaveLength(1);
    const around: string[] = [];
    for (let n = label[0].parent; n; n = n.parent) if (opening(n)) around.push(...classes(opening(n)!));
    expect(around).toContain("rail");
    expect(around.filter((c) => scrolls.has(c))).toEqual([]);
    // …while the rest of the rail still scrolls, so a long list stays reachable.
    expect(all.some((n) => classes(opening(n)!).some((c) => scrolls.has(c)))).toBe(true);
  });
});
