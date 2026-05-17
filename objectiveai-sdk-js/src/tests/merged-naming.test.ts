/**
 * Naming convention test: every exported function ending in "Merged" or "MergedList"
 * must return a tuple [T, boolean], and the function name must be camelCase(T) + "Merged"
 * (or "MergedList").
 */
import { describe, it, expect } from "vitest";
import * as ts from "typescript";
import * as path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

function pascalToCamel(s: string): string {
  return s[0].toLowerCase() + s.slice(1);
}

interface MergedFn {
  name: string;
  file: string;
  returnTypeName: string | null;
}

/**
 * Get the type alias name from a type node in the source (the syntactic return type).
 * This avoids the TS checker expanding the type to its structural form.
 */
function getTypeNameFromAnnotation(
  node: ts.FunctionDeclaration,
): string | null {
  const returnTypeNode = node.type;
  if (!returnTypeNode) return null;

  // Return type should be [T, boolean] — a tuple literal type
  if (!ts.isTupleTypeNode(returnTypeNode)) return null;

  const elements = returnTypeNode.elements;
  if (elements.length < 2) return null;

  const firstElement = elements[0];

  // The first element could be a type reference (named type) or array type
  return extractTypeName(firstElement);
}

function extractTypeName(node: ts.TypeNode): string | null {
  // Simple type reference: AgentCompletionsResponseUsage
  if (ts.isTypeReferenceNode(node)) {
    return entityNameToString(node.typeName);
  }
  // Array type: T[]
  if (ts.isArrayTypeNode(node)) {
    const inner = extractTypeName(node.elementType);
    if (inner) return inner + "[]";
  }
  // Union type: string | number
  if (ts.isUnionTypeNode(node)) {
    // For (string | number)[], the parent handles it
    // For inline unions like in scores, return the text
    return node.getText();
  }
  // Parenthesized type: (string | number)
  if (ts.isParenthesizedTypeNode(node)) {
    return extractTypeName(node.type);
  }
  // Keyword types
  if (node.kind === ts.SyntaxKind.StringKeyword) return "string";
  if (node.kind === ts.SyntaxKind.NumberKeyword) return "number";

  return null;
}

function entityNameToString(name: ts.EntityName): string {
  if (ts.isIdentifier(name)) return name.text;
  return entityNameToString(name.left) + "." + name.right.text;
}

function findMergedFunctions(): MergedFn[] {
  const configPath = path.resolve(__dirname, "../../tsconfig.json");
  const configFile = ts.readConfigFile(configPath, ts.sys.readFile);
  const parsedConfig = ts.parseJsonConfigFileContent(
    configFile.config,
    ts.sys,
    path.dirname(configPath),
  );

  const program = ts.createProgram(
    parsedConfig.fileNames,
    parsedConfig.options,
  );
  const checker = program.getTypeChecker();
  const results: MergedFn[] = [];

  for (const sourceFile of program.getSourceFiles()) {
    if (sourceFile.isDeclarationFile) continue;
    const normalizedPath = sourceFile.fileName.replace(/\\/g, "/");
    if (!normalizedPath.includes("objectiveai-sdk-js/src/")) continue;
    if (normalizedPath.includes(".test.")) continue;
    if (!normalizedPath.includes("Merged")) continue;

    ts.forEachChild(sourceFile, (node) => {
      if (
        !ts.isFunctionDeclaration(node) ||
        !node.name ||
        !node.modifiers?.some(
          (m) => m.kind === ts.SyntaxKind.ExportKeyword,
        )
      ) {
        return;
      }

      const name = node.name.text;
      if (!name.endsWith("Merged") && !name.endsWith("MergedList")) return;

      const returnTypeName = getTypeNameFromAnnotation(node);

      const relFile = path
        .relative(path.resolve(__dirname, "../.."), sourceFile.fileName)
        .replace(/\\/g, "/");

      results.push({ name, file: relFile, returnTypeName });
    });
  }

  return results;
}

describe("Merged function naming conventions", () => {
  const fns = findMergedFunctions();

  it("should find merged functions", () => {
    expect(fns.length).toBeGreaterThan(0);
  });

  for (const fn of fns) {
    it(`${fn.name} should return a tuple [T, boolean]`, () => {
      // *FieldsMerged is a helper that returns a plain object, not a tuple
      if (fn.name.endsWith("FieldsMerged")) return;
      expect(fn.returnTypeName).not.toBeNull();
    });

    it(`${fn.name} should match its return type name`, () => {
      if (fn.returnTypeName === null) return;

      const isList = fn.name.endsWith("MergedList");
      const suffix = isList ? "MergedList" : "Merged";

      // For MergedList, the return type is T[] — extract T
      let typeName = fn.returnTypeName;
      if (isList && typeName.endsWith("[]")) {
        typeName = typeName.slice(0, -2);
      }

      const expectedName = pascalToCamel(typeName) + suffix;
      expect(fn.name).toBe(expectedName);
    });
  }
});
