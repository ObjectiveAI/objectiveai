/**
 * Ensures every export from every non-test .ts file in src/ is reachable
 * from the root index.ts re-export chain.
 *
 * Walks every .ts file under src/, collects all exported names, then checks
 * that every name is also exported from src/index.ts (transitively via
 * `export * from` chains).
 */
import { describe, it, expect } from "vitest";
import * as ts from "typescript";
import * as path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const srcDir = path.resolve(__dirname, "..");

/** Files matching any of these patterns (against forward-slashed absolute path) are excluded. */
const WHITELIST: RegExp[] = [
  /^(?!.*objectiveai-sdk-js\/src\/)/,
  /\.d\.ts$/,
  /\.test\./,
  /\/index\.ts$/,
  /\/generatedIndex\.ts$/,
  /\/tests\//,
  /\/wasm\//,
  /\/jsonValue\.ts$/,
  /\/mergeTestUtil\.ts$/,
];

function createProgram() {
  const configPath = path.resolve(srcDir, "../tsconfig.json");
  const configFile = ts.readConfigFile(configPath, ts.sys.readFile);
  const parsedConfig = ts.parseJsonConfigFileContent(
    configFile.config,
    ts.sys,
    path.dirname(configPath),
  );
  return ts.createProgram(parsedConfig.fileNames, parsedConfig.options);
}

/** Collect all export names from a source file (types, interfaces, functions, consts, enums, classes) */
function getExportedNames(sourceFile: ts.SourceFile, checker: ts.TypeChecker): string[] {
  const symbol = checker.getSymbolAtLocation(sourceFile);
  if (!symbol) return [];
  const exports = checker.getExportsOfModule(symbol);
  return exports.map((e) => e.getName());
}

function normalize(fileName: string): string {
  return fileName.replace(/\\/g, "/");
}

describe("all exports available from root index.ts", () => {
  const program = createProgram();
  const checker = program.getTypeChecker();

  // Find the root index.ts source file
  const indexFile = program
    .getSourceFiles()
    .find((sf) => normalize(sf.fileName).endsWith("objectiveai-sdk-js/src/index.ts"));

  if (!indexFile) {
    it("should find src/index.ts", () => {
      expect(indexFile).toBeDefined();
    });
    return;
  }

  // Get everything exported from root index.ts (transitively)
  const rootExports = new Set(getExportedNames(indexFile, checker));

  // Collect all source files that should have their exports propagated
  const sourceFiles = program.getSourceFiles().filter((sf) => {
    const norm = normalize(sf.fileName);
    if (WHITELIST.some((re) => re.test(norm))) return false;
    return true;
  });

  it("should find source files to check", () => {
    expect(sourceFiles.length).toBeGreaterThan(0);
  });

  it("should have root exports", () => {
    expect(rootExports.size).toBeGreaterThan(0);
  });

  // For each source file, check that every export is available from root
  const missing: { name: string; file: string }[] = [];

  for (const sf of sourceFiles) {
    const exports = getExportedNames(sf, checker);
    const relFile = path
      .relative(path.resolve(srcDir, ".."), sf.fileName)
      .replace(/\\/g, "/");

    for (const name of exports) {
      if (!rootExports.has(name)) {
        missing.push({ name, file: relFile });
      }
    }
  }

  for (const { name, file } of missing) {
    it(`${name} (from ${file}) should be importable from root index.ts`, () => {
      expect(rootExports.has(name)).toBe(true);
    });
  }

  if (missing.length === 0) {
    it("all exports are available from root index.ts", () => {
      expect(missing).toEqual([]);
    });
  }
});
