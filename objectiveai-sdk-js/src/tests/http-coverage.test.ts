/**
 * HTTP function coverage test: every Rust http.rs file must have a
 * corresponding TypeScript http.ts file at the same path, exporting
 * functions whose names include the full module path.
 *
 * Naming convention mirrors JSON schema titles but in camelCase:
 *   Schema type in agent/  :  AgentListAgent        (PascalCase)
 *   HTTP fn in agent/http  :  agentListAgents        (camelCase)
 *
 * The full name is the module path segments + snake_to_PascalCase function
 * name, joined and lowercased at the first character.
 *
 * Streaming/unary pairs (e.g. create_foo_unary / create_foo_streaming)
 * map to a single TypeScript export (e.g. moduleCreateFoo).
 */
import { describe, it, expect } from "vitest";
import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const RUST_SRC = path.resolve(__dirname, "../../../objectiveai-sdk-rs/src");
const TS_SRC = path.resolve(__dirname, "..");

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** "list_agents" → "ListAgents" */
function snakeToPascal(name: string): string {
  return name
    .split("_")
    .map((s) => s[0].toUpperCase() + s.slice(1))
    .join("");
}

/** "AgentListAgents" → "agentListAgents" */
function pascalToCamel(name: string): string {
  return name[0].toLowerCase() + name.slice(1);
}

/**
 * Build the full camelCase export name from module path + snake_case fn name.
 *
 *   modulePath: "agent.completions"
 *   fnName:     "create_agent_completion"
 *   → PascalCase segments: ["Agent", "Completions", "Create", "Agent", "Completion"]
 *   → joined:   "AgentCompletionsCreateAgentCompletion"
 *   → camel:    "agentCompletionsCreateAgentCompletion"
 */
function buildFullName(modulePath: string, snakeFnName: string): string {
  const pathPascal = modulePath
    .split(".")
    .map((s) => s[0].toUpperCase() + s.slice(1));
  const fnPascal = snakeToPascal(snakeFnName);
  return pascalToCamel([...pathPascal, fnPascal].join(""));
}

/**
 * Walks a source tree looking for files named `http.{ext}`.
 * Returns a map from dot-separated module path to file contents.
 *
 *   src/agent/completions/http.rs  →  "agent.completions"
 */
function findHttpFiles(root: string, ext: string): Map<string, string> {
  const result = new Map<string, string>();
  function walk(dir: string, segments: string[]) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      if (entry.isDirectory()) {
        walk(path.join(dir, entry.name), [...segments, entry.name]);
      } else if (entry.name === `http.${ext}`) {
        const modulePath = segments.join(".");
        result.set(
          modulePath,
          fs.readFileSync(path.join(dir, entry.name), "utf-8"),
        );
      }
    }
  }
  walk(root, []);
  return result;
}

/** Extract `pub async fn <name>` from Rust source. */
function extractRustFunctions(content: string): string[] {
  const regex = /pub\s+async\s+fn\s+(\w+)/g;
  const names: string[] = [];
  let match;
  while ((match = regex.exec(content)) !== null) {
    names.push(match[1]);
  }
  return names;
}

/** Extract `export function <name>` from TypeScript source (deduplicated for overloads). */
function extractTsFunctions(content: string): string[] {
  const regex = /export\s+function\s+(\w+)/g;
  const names = new Set<string>();
  let match;
  while ((match = regex.exec(content)) !== null) {
    names.add(match[1]);
  }
  return [...names];
}

/**
 * Streaming/unary pairs in Rust (e.g. `create_foo_unary` and
 * `create_foo_streaming`) map to a single TypeScript function.
 * Strip the suffix to get the base name before comparison.
 */
function stripStreamingSuffix(name: string): string {
  return name.replace(/_(unary|streaming)$/, "");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("HTTP function coverage", () => {
  const rustHttpFiles = findHttpFiles(RUST_SRC, "rs");
  const tsHttpFiles = findHttpFiles(TS_SRC, "ts");

  it("every Rust http.rs has a corresponding TypeScript http.ts", () => {
    const missing: string[] = [];
    for (const modulePath of rustHttpFiles.keys()) {
      if (!tsHttpFiles.has(modulePath)) {
        missing.push(modulePath);
      }
    }
    expect(
      missing,
      `Missing http.ts for: ${missing.join(", ")}`,
    ).toEqual([]);
  });

  it("every Rust http function has a corresponding TypeScript export", () => {
    const errors: string[] = [];

    for (const [modulePath, rustContent] of rustHttpFiles) {
      const tsContent = tsHttpFiles.get(modulePath);
      if (!tsContent) continue;

      const rustFns = extractRustFunctions(rustContent);
      const tsFnNames = new Set(extractTsFunctions(tsContent));

      // Deduplicate streaming/unary pairs to their base name
      const rustBaseNames = [
        ...new Set(rustFns.map(stripStreamingSuffix)),
      ];

      for (const rustBase of rustBaseNames) {
        const expectedTsName = buildFullName(modulePath, rustBase);
        if (!tsFnNames.has(expectedTsName)) {
          errors.push(
            `${modulePath}: expected export "${expectedTsName}" (from Rust "${rustBase}")`,
          );
        }
      }
    }

    expect(
      errors,
      `Missing TypeScript exports:\n  ${errors.join("\n  ")}`,
    ).toEqual([]);
  });

  it("every TypeScript http export has a corresponding Rust function", () => {
    const errors: string[] = [];

    for (const [modulePath, tsContent] of tsHttpFiles) {
      const rustContent = rustHttpFiles.get(modulePath);
      if (!rustContent) {
        errors.push(`${modulePath} (http.ts exists without http.rs)`);
        continue;
      }

      const tsFns = extractTsFunctions(tsContent);
      const rustFns = extractRustFunctions(rustContent);
      const expectedTsNames = new Set(
        [...new Set(rustFns.map(stripStreamingSuffix))].map((base) =>
          buildFullName(modulePath, base),
        ),
      );

      for (const tsFn of tsFns) {
        if (!expectedTsNames.has(tsFn)) {
          errors.push(
            `${modulePath}: unexpected export "${tsFn}" has no Rust counterpart`,
          );
        }
      }
    }

    expect(
      errors,
      `Extra TypeScript exports without Rust counterpart:\n  ${errors.join("\n  ")}`,
    ).toEqual([]);
  });
});
