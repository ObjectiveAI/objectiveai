import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { createElement, type ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import ts from "typescript";
import { describe, expect, it } from "vitest";
import { Button, Card, Chip, Row, SectionHead } from "./components/ui";

const SRC = fileURLToPath(new URL(".", import.meta.url));
const UI = join(SRC, "components", "ui.tsx");
const KINDS = ["primary", "secondary", "tertiary", "danger", "icon"];

/** Every .tsx file under src/ but ui.tsx, which is where the pressable parts are made. */
function screens(): string[] {
  const out: string[] = [];
  const walk = (dir: string) => {
    for (const n of readdirSync(dir).sort()) {
      const p = join(dir, n);
      if (statSync(p).isDirectory()) walk(p);
      else if (n.endsWith(".tsx") && !/\.test\.tsx$/.test(n) && p !== UI) out.push(p);
    }
  };
  walk(SRC);
  return out;
}
const parse = (file: string) => ts.createSourceFile(file, readFileSync(file, "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
const at = (file: string, source: ts.SourceFile, n: ts.Node) => `${relative(SRC, file)}:${source.getLineAndCharacterOfPosition(n.getStart(source)).line + 1}`;

/** Each JSX element in a file, with its tag name. */
function elements(file: string): { tag: string; el: ts.JsxOpeningLikeElement; where: string }[] {
  const source = parse(file);
  const out: { tag: string; el: ts.JsxOpeningLikeElement; where: string }[] = [];
  const visit = (n: ts.Node) => {
    if (ts.isJsxOpeningElement(n) || ts.isJsxSelfClosingElement(n)) out.push({ tag: n.tagName.getText(source), el: n, where: at(file, source, n) });
    ts.forEachChild(n, visit);
  };
  visit(source);
  return out;
}

/** The class words each className in a file spells as plain text, with where. */
function classWords(file: string): { word: string; where: string }[] {
  const source = parse(file);
  const out: { word: string; where: string }[] = [];
  const visit = (n: ts.Node, inClass: boolean) => {
    if (ts.isJsxAttribute(n) && n.name.getText(source) === "className") inClass = true;
    if (inClass && (ts.isStringLiteral(n) || ts.isNoSubstitutionTemplateLiteral(n) || ts.isTemplateHead(n) || ts.isTemplateMiddle(n) || ts.isTemplateTail(n))) {
      for (const word of n.text.split(/\s+/).filter(Boolean)) out.push({ word, where: at(file, source, n) });
    }
    ts.forEachChild(n, (c) => visit(c, inClass));
  };
  visit(source, false);
  return out;
}

/** The rules app.css and theme.css set, as "selector { prop: value }" lines (comments dropped). */
function rules(name: string): Map<string, Map<string, string>> {
  const text = readFileSync(join(SRC, name), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  const out = new Map<string, Map<string, string>>();
  for (const m of text.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    for (const selector of m[1].split(",").map((s) => s.trim().replace(/\s+/g, " "))) {
      const props = out.get(selector) ?? new Map<string, string>();
      for (const d of m[2].split(";")) {
        const colon = d.indexOf(":");
        if (colon > 0) props.set(d.slice(0, colon).trim(), d.slice(colon + 1).trim());
      }
      out.set(selector, props);
    }
  }
  return out;
}
const app = rules("app.css");
const theme = rules("theme.css");
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const html = (type: (props: any) => ReactNode, props: Record<string, unknown>, ...children: string[]) => renderToStaticMarkup(createElement(type, props, ...children));

describe("pressable parts come from ui.tsx", () => {
  it("no raw <button> outside ui.tsx", () => {
    const raw = screens().flatMap((f) => elements(f).filter((e) => e.tag === "button").map((e) => e.where));
    expect(raw).toEqual([]);
  });

  it("no 'link' or 'btn' class outside ui.tsx, nor the list heads SectionHead replaced", () => {
    const off = screens().flatMap((f) => classWords(f).filter((c) => c.word === "link" || c.word === "btn" || c.word.startsWith("btn-") || c.word === "list-head").map((c) => `${c.where} ${c.word}`));
    expect(off).toEqual([]);
  });
});

describe("Button", () => {
  it("has exactly five kinds: primary, secondary, tertiary, danger and icon", () => {
    const source = parse(UI);
    let kinds: string[] | null = null;
    const visit = (n: ts.Node) => {
      if (ts.isTypeAliasDeclaration(n) && n.name.text === "ButtonKind") {
        const parts = ts.isUnionTypeNode(n.type) ? n.type.types : [n.type];
        kinds = parts.map((p) => (ts.isLiteralTypeNode(p) && ts.isStringLiteral(p.literal) ? p.literal.text : `(not a word: ${p.getText(source)})`));
      }
      ts.forEachChild(n, visit);
    };
    visit(source);
    expect(kinds, "ui.tsx declares type ButtonKind").not.toBeNull();
    expect([...kinds!].sort()).toEqual([...KINDS].sort());
  });

  it("each kind has its look in app.css", () => {
    for (const k of KINDS) expect(app.has(`.btn-${k}`), `.btn-${k}`).toBe(true);
    for (const k of KINDS.filter((k) => k !== "icon")) {
      const html_ = html(Button, { kind: k, onClick: () => {} }, "Go");
      expect(html_).toContain(`btn-${k}`);
      expect(html_).toContain('type="button"');
    }
  });

  it("an icon button names itself for screen readers", () => {
    const out = html(Button, { kind: "icon", label: "Close", onClick: () => {} });
    expect(out).toMatch(/^<button [^>]*aria-label="Close"/);
    expect(out).toContain("btn-icon");
    // An icon button with no label doesn't type-check (tsc, which the build runs, reads this line).
    // @ts-expect-error an icon button must have a label
    const nameless: Parameters<typeof Button>[0] = { kind: "icon", onClick: () => {} };
    expect(nameless.kind).toBe("icon");
  });
});

describe("Chip", () => {
  it("a label chip is words, not a button, and draws no edge", () => {
    expect(html(Chip, { tone: "ok" }, "Done")).toMatch(/^<span class="chip chip-ok"/);
    expect(app.get(".chip")?.get("border")).toBe("1px solid transparent");
  });

  it("a pressable chip is a button with an edge", () => {
    const out = html(Chip, { onClick: () => {} }, "A room");
    expect(out).toMatch(/^<button type="button" class="chip chip-press"/);
    expect(app.get(".chip-press")?.get("border-color")).toBe("var(--line)");
  });
});

describe("Card", () => {
  it("is a box of words, or a button when it can be pressed", () => {
    expect(html(Card, { tone: "bad" }, "x")).toMatch(/^<div class="card card-bad"/);
    expect(html(Card, { onClick: () => {} }, "x")).toMatch(/^<button type="button" class="card card-press"/);
  });
});

describe("Row", () => {
  it("is a button, and says which row is where you are", () => {
    expect(html(Row, { on: true, onClick: () => {}, className: "tree-row" }, "a")).toMatch(/^<button type="button" class="nav-row tree-row on" aria-current="true"/);
    expect(html(Row, { onClick: () => {} }, "a")).toMatch(/^<button type="button" class="nav-row">/);
    // A row that picks one of several (a kind of agent) says it's pressed, not where you are.
    expect(html(Row, { on: true, choice: true, onClick: () => {} }, "a")).toMatch(/^<button type="button" class="nav-row on" aria-pressed="true"/);
  });
});

describe("SectionHead", () => {
  it("heads a section with its title and note, at the level asked", () => {
    expect(html(SectionHead, { title: "Rules", note: "Set by the host." })).toBe('<header class="section-head"><div class="section-titles"><h2>Rules</h2><p class="muted">Set by the host.</p></div></header>');
    expect(html(SectionHead, { title: "Saved", level: 3, small: true })).toBe('<header class="section-head section-head-small"><div class="section-titles"><h3>Saved</h3></div></header>');
  });
});

describe("the 28px floor", () => {
  it("--hit-min is 28px, and every pressable part is at least that tall", () => {
    expect(theme.get(":root")?.get("--hit-min")).toBe("28px");
    for (const sel of [".btn", ".chip-press", ".nav-row", ".card-press"]) expect(app.get(sel)?.get("min-height"), sel).toBe("var(--hit-min)");
    expect(app.get(".btn-icon")?.get("min-width")).toBe("var(--hit-min)");
  });
});
