const { spawnSync } = require("child_process");
const {
  mkdirSync,
  copyFileSync,
  writeFileSync,
  rmSync,
  existsSync,
} = require("fs");
const path = require("path");

// Paths
const jsRoot = process.cwd(); // objectiveai-js
const repoRoot = path.resolve(jsRoot, ".."); // objectiveai
const wasmDir = path.join(repoRoot, "objectiveai", "objectiveai-wasm-js");
const outDir = path.join(jsRoot, "src", "wasm");
const nodejsOutDir = path.join(outDir, "nodejs");
const bundlerOutDir = path.join(outDir, "bundler");

// Build targets configuration
const targets = [
  { name: "nodejs", outDir: "pkg-nodejs" },
  { name: "bundler", outDir: "pkg-bundler" },
];

// 0. Clean up old files in src/wasm (except .gitignore)
const oldFiles = [
  "objectiveai_wasm_js.js",
  "objectiveai_wasm_js.d.ts",
  "objectiveai_wasm_js_bg.js",
  "objectiveai_wasm_js_bg.wasm",
  "objectiveai_wasm_js_bg.wasm.d.ts",
];

for (const file of oldFiles) {
  const filePath = path.join(outDir, file);
  if (existsSync(filePath)) {
    rmSync(filePath);
    console.log(`✓ Removed old file: ${file}`);
  }
}

// Clean nodejs and bundler directories
if (existsSync(nodejsOutDir)) {
  rmSync(nodejsOutDir, { recursive: true });
}
if (existsSync(bundlerOutDir)) {
  rmSync(bundlerOutDir, { recursive: true });
}

// 1. Build both wasm-pack targets
for (const target of targets) {
  console.log(`⚙ Building wasm-pack target: ${target.name}`);

  const result = spawnSync(
    "wasm-pack",
    ["build", "--target", target.name, "--release", "--out-dir", target.outDir],
    {
      cwd: wasmDir,
      stdio: "inherit",
      shell: process.platform === "win32",
    }
  );

  if (result.status !== 0) {
    console.error(`Failed to build ${target.name} target`);
    process.exit(result.status ?? 1);
  }
}

// 2. Create destination directories
mkdirSync(nodejsOutDir, { recursive: true });
mkdirSync(bundlerOutDir, { recursive: true });

// 3. Copy nodejs target files (rename .js to .cjs for proper CJS handling)
const nodejsPkgDir = path.join(wasmDir, "pkg-nodejs");

// Copy and rename the main JS file to .cjs
copyFileSync(
  path.join(nodejsPkgDir, "objectiveai_wasm_js.js"),
  path.join(nodejsOutDir, "objectiveai_wasm_js.cjs")
);
console.log("✓ Copied nodejs/objectiveai_wasm_js.cjs");

// Copy other files
const nodejsOtherFiles = [
  "objectiveai_wasm_js.d.ts",
  "objectiveai_wasm_js_bg.wasm",
  "objectiveai_wasm_js_bg.wasm.d.ts",
];

for (const file of nodejsOtherFiles) {
  const from = path.join(nodejsPkgDir, file);
  const to = path.join(nodejsOutDir, file);
  copyFileSync(from, to);
  console.log(`✓ Copied nodejs/${file}`);
}

// 4. Copy bundler target files
const bundlerFiles = [
  "objectiveai_wasm_js.js",
  "objectiveai_wasm_js.d.ts",
  "objectiveai_wasm_js_bg.js",
  "objectiveai_wasm_js_bg.wasm",
  "objectiveai_wasm_js_bg.wasm.d.ts",
];

const bundlerPkgDir = path.join(wasmDir, "pkg-bundler");
for (const file of bundlerFiles) {
  const from = path.join(bundlerPkgDir, file);
  const to = path.join(bundlerOutDir, file);
  copyFileSync(from, to);
  console.log(`✓ Copied bundler/${file}`);
}

// 5. Create the ESM loader with runtime detection
const esmLoader = `// Auto-generated ESM loader with runtime detection
// Detects Node.js vs Browser and loads appropriate WASM implementation

const isNode =
  typeof process !== "undefined" &&
  process.versions != null &&
  process.versions.node != null;

let wasm;

if (isNode) {
  // Node.js: dynamically import the nodejs target (CJS module with .cjs extension)
  // Node.js ESM can import CJS modules via dynamic import
  wasm = await import("./nodejs/objectiveai_wasm_js.cjs");
} else {
  // Browser: import the bundler target
  // Bundlers (Webpack/Vite) will process this and handle the .wasm import
  wasm = await import("./bundler/objectiveai_wasm_js.js");
}

// Re-export all WASM functions
export const validateEnsembleLlm = wasm.validateEnsembleLlm ?? wasm.default?.validateEnsembleLlm;
export const validateEnsemble = wasm.validateEnsemble ?? wasm.default?.validateEnsemble;
export const compileFunctionTasks = wasm.compileFunctionTasks ?? wasm.default?.compileFunctionTasks;
export const compileFunctionOutput = wasm.compileFunctionOutput ?? wasm.default?.compileFunctionOutput;
export const promptId = wasm.promptId ?? wasm.default?.promptId;
export const toolsId = wasm.toolsId ?? wasm.default?.toolsId;
export const vectorResponseId = wasm.vectorResponseId ?? wasm.default?.vectorResponseId;
`;

writeFileSync(path.join(outDir, "loader.js"), esmLoader);
console.log("✓ Created loader.js (ESM)");

// 6. Create the CJS loader (assumes Node.js runtime)
const cjsLoader = `// Auto-generated CJS loader for Node.js
// CJS is only used in Node.js, so we directly use the nodejs target

module.exports = require("./nodejs/objectiveai_wasm_js.cjs");
`;

writeFileSync(path.join(outDir, "loader.cjs"), cjsLoader);
console.log("✓ Created loader.cjs (CJS)");

// 7. Create TypeScript declaration for the loader
const loaderDts = `// Auto-generated type declarations for WASM loader

export function validateEnsembleLlm(llm: any): any;
export function validateEnsemble(ensemble: any): any;
export function compileFunctionTasks(_function: any, input: any): any;
export function compileFunctionOutput(_function: any, input: any, task_outputs: any): any;
export function promptId(prompt: any): string;
export function toolsId(tools: any): string;
export function vectorResponseId(response: any): string;
`;

writeFileSync(path.join(outDir, "loader.d.ts"), loaderDts);
console.log("✓ Created loader.d.ts");

console.log("\n✅ WASM installation complete");
