/**
 * Validates that every wasm.ts wrapper file exports functions matching the
 * naming convention `wasm{PascalPath}{PascalName}`, where PascalName is the
 * PascalCase of the corresponding wasm_bindgen function name from lib.rs.
 *
 * Also asserts a 1:1 mapping between wasm_bindgen functions and TS exports.
 */
import { describe, it, expect } from "vitest";
import * as ts from "typescript";
import * as path from "path";
import * as fs from "fs";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const srcDir = path.resolve(__dirname, "..");

// ---------------------------------------------------------------------------
// 1. Collect exported function names from all src/**/wasm.ts files
// ---------------------------------------------------------------------------

interface WasmExport {
  name: string;
  /** Relative path from src/ to the wasm.ts file, forward-slashed */
  relPath: string;
}

function findWasmTsExports(): WasmExport[] {
  const results: WasmExport[] = [];

  function walk(dir: string) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      if (entry.isDirectory()) {
        if (entry.name === "node_modules" || entry.name === "wasm") continue;
        walk(path.join(dir, entry.name));
      } else if (entry.name === "wasm.ts") {
        const filePath = path.join(dir, entry.name);
        const relPath = path.relative(srcDir, filePath).replace(/\\/g, "/");

        const sourceText = fs.readFileSync(filePath, "utf-8");
        const sourceFile = ts.createSourceFile(
          filePath,
          sourceText,
          ts.ScriptTarget.Latest,
          true,
        );

        ts.forEachChild(sourceFile, (node) => {
          if (
            ts.isFunctionDeclaration(node) &&
            node.name &&
            node.modifiers?.some(
              (m) => m.kind === ts.SyntaxKind.ExportKeyword,
            )
          ) {
            results.push({ name: node.name.text, relPath });
          }
        });
      }
    }
  }

  walk(srcDir);
  return results;
}

// ---------------------------------------------------------------------------
// 2. Collect public wasm_bindgen function names from lib.rs
// ---------------------------------------------------------------------------

function findBindgenFunctions(): string[] {
  const libRsPath = path.resolve(
    __dirname,
    "../../../objectiveai-rs-wasm-js/src/lib.rs",
  );
  const source = fs.readFileSync(libRsPath, "utf-8");
  const names: string[] = [];

  const lines = source.split("\n");
  let sawBindgen = false;
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("//")) continue;
    if (trimmed === "#[wasm_bindgen]") {
      sawBindgen = true;
      continue;
    }
    if (sawBindgen) {
      const m = trimmed.match(/^pub fn (\w+)\(/);
      if (m) {
        names.push(m[1]);
        sawBindgen = false;
      }
    }
  }
  return names;
}

// ---------------------------------------------------------------------------
// 3. Helpers
// ---------------------------------------------------------------------------

/** Convert a directory path like "agent/completions/response/streaming" to "AgentCompletionsResponseStreaming" */
function pathToPascal(relPath: string): string {
  const dir = relPath.replace(/\/wasm\.ts$/, "");
  return dir
    .split("/")
    .map((seg) => seg.replace(/(^|_)([a-z])/g, (_, __, c) => c.toUpperCase()))
    .join("");
}

/** Convert a camelCase function name to PascalCase */
function toPascal(name: string): string {
  return name[0].toUpperCase() + name.slice(1);
}

/** Build the expected TS export name for a bindgen function in a given wasm.ts file */
function expectedExportName(bindgenName: string, relPath: string): string {
  return "wasm" + pathToPascal(relPath) + toPascal(bindgenName);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("wasm.ts export naming conventions", () => {
  const wasmExports = findWasmTsExports();
  const bindgenFns = findBindgenFunctions();

  // Build a set of all TS export names for lookup
  const exportNames = new Set(wasmExports.map((e) => e.name));

  // Build a map from export name → relPath for reverse lookup
  const exportToPath = new Map(wasmExports.map((e) => [e.name, e.relPath]));

  it("should find wasm.ts exports", () => {
    expect(wasmExports.length).toBeGreaterThan(0);
  });

  it("should find wasm_bindgen functions", () => {
    expect(bindgenFns.length).toBeGreaterThan(0);
  });

  it("number of wasm.ts exports should equal number of wasm_bindgen functions", () => {
    expect(wasmExports.length).toBe(bindgenFns.length);
  });

  // For each TS export, verify: name = "wasm" + PascalPath + PascalName,
  // where PascalName is toPascal() of exactly one bindgen function name,
  // and PascalPath and PascalName do not overlap.
  for (const exp of wasmExports) {
    it(`${exp.name} should start with "wasm"`, () => {
      expect(exp.name.startsWith("wasm")).toBe(true);
    });

    it(`${exp.name} should equal wasm{PascalPath}{PascalName} for exactly one bindgen function`, () => {
      const pascalPath = pathToPascal(exp.relPath);
      const afterWasm = exp.name.slice(4); // strip "wasm"

      // PascalPath must appear immediately after "wasm"
      expect(afterWasm.startsWith(pascalPath)).toBe(true);

      // PascalName is the remainder — must be non-empty and PascalCase
      const pascalName = afterWasm.slice(pascalPath.length);
      expect(pascalName.length).toBeGreaterThan(0);
      expect(pascalName[0]).toBe(pascalName[0].toUpperCase());

      // PascalName must be exactly toPascal(bindgenName) for some bindgen function
      const matchingBindgen = bindgenFns.filter(
        (fn) => toPascal(fn) === pascalName,
      );
      expect(matchingBindgen).toHaveLength(1);
    });
  }

  // For each bindgen function, verify exactly one TS export matches it
  for (const bindgenName of bindgenFns) {
    it(`bindgen ${bindgenName} should have exactly one matching TS export`, () => {
      const matches = wasmExports.filter(
        (exp) => exp.name === expectedExportName(bindgenName, exp.relPath),
      );
      expect(matches).toHaveLength(1);
    });
  }
});
