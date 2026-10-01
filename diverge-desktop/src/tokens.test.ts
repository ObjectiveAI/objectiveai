import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import { describe, expect, it } from "vitest";

/** One declaration in a style sheet, with the selector and at-rules it sits under. */
type Decl = { at: string; line: number; selector: string; within: string[]; prop: string; value: string };

const SRC = fileURLToPath(new URL(".", import.meta.url));

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

describe("tokens", () => {
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
