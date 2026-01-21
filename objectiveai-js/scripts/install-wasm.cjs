const { spawnSync } = require("child_process");
const { mkdirSync, readFileSync, writeFileSync, rmSync, existsSync } = require("fs");
const path = require("path");

// Paths
const jsRoot = process.cwd(); // objectiveai-js
const repoRoot = path.resolve(jsRoot, ".."); // objectiveai
const wasmDir = path.join(repoRoot, "objectiveai-rs-wasm-js");
const outDir = path.join(jsRoot, "src", "wasm");

// Clean up old files
if (existsSync(outDir)) {
  rmSync(outDir, { recursive: true });
}
mkdirSync(outDir, { recursive: true });

// 1. Build nodejs target (we only need one target now - just need the glue code and .wasm)
console.log("⚙ Building wasm-pack target: nodejs");

const result = spawnSync(
  "wasm-pack",
  ["build", "--target", "nodejs", "--release", "--out-dir", "pkg-nodejs"],
  {
    cwd: wasmDir,
    stdio: "inherit",
    shell: process.platform === "win32",
  }
);

if (result.status !== 0) {
  console.error("Failed to build nodejs target");
  process.exit(result.status ?? 1);
}

// 2. Read the generated files
const nodejsPkgDir = path.join(wasmDir, "pkg-nodejs");
const glueCode = readFileSync(
  path.join(nodejsPkgDir, "objectiveai_wasm_js.js"),
  "utf-8"
);
const wasmBinary = readFileSync(
  path.join(nodejsPkgDir, "objectiveai_wasm_js_bg.wasm")
);
const wasmBase64 = wasmBinary.toString("base64");

console.log(`✓ WASM binary size: ${wasmBinary.length} bytes`);
console.log(`✓ WASM base64 size: ${wasmBase64.length} chars`);

// 3. Modify the glue code to use embedded base64 instead of fs.readFileSync
// Find and replace the WASM loading code at the end
const fsLoadPattern = /const wasmPath[\s\S]*?wasm\.__wbindgen_start\(\);/;

const universalLoaderCode = `
// Universal base64-encoded WASM loader
// Works in Node.js (ESM/CJS) and browsers without bundler configuration
const WASM_BASE64 = "${wasmBase64}";

function decodeBase64(base64) {
  if (typeof Buffer !== 'undefined') {
    // Node.js
    return Buffer.from(base64, 'base64');
  } else {
    // Browser
    const binaryString = atob(base64);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes;
  }
}

const wasmBytes = decodeBase64(WASM_BASE64);
const wasmModule = new WebAssembly.Module(wasmBytes);
const wasm = exports.__wasm = new WebAssembly.Instance(wasmModule, imports).exports;

wasm.__wbindgen_start();`;

// Also need to handle the decodeText function which uses TextDecoder
// The nodejs target might use a Node.js-specific approach
let modifiedGlue = glueCode.replace(fsLoadPattern, universalLoaderCode);

// Check if there's a require('util') for TextDecoder - make it universal
if (modifiedGlue.includes("require('util')")) {
  // Replace Node.js TextDecoder with universal version
  modifiedGlue = modifiedGlue.replace(
    /const \{ TextDecoder \} = require\('util'\);/g,
    "const TextDecoder = typeof globalThis.TextDecoder !== 'undefined' ? globalThis.TextDecoder : require('util').TextDecoder;"
  );
}

// 4. Write the CJS version (the modified glue code is already CJS)
writeFileSync(path.join(outDir, "loader.cjs"), modifiedGlue);
console.log("✓ Created loader.cjs (universal CJS loader)");

// 5. Create ESM version by wrapping the CJS code in an IIFE
// This avoids symbol conflicts with the function declarations inside
const esmLoader = `// Universal ESM loader with embedded base64 WASM
// Works in Node.js and browsers without bundler configuration

const _wasm = (() => {
  const exports = {};
  const module = { exports };
  ${modifiedGlue
    .replace("imports['__wbindgen_placeholder__'] = module.exports;", "imports['__wbindgen_placeholder__'] = exports;")
  }
  return exports;
})();

export const validateEnsembleLlm = _wasm.validateEnsembleLlm;
export const validateEnsemble = _wasm.validateEnsemble;
export const compileFunctionTasks = _wasm.compileFunctionTasks;
export const compileFunctionOutput = _wasm.compileFunctionOutput;
export const promptId = _wasm.promptId;
export const toolsId = _wasm.toolsId;
export const vectorResponseId = _wasm.vectorResponseId;
`;

writeFileSync(path.join(outDir, "loader.js"), esmLoader);
console.log("✓ Created loader.js (universal ESM loader)");

// 6. Copy type declarations
const dtsContent = readFileSync(
  path.join(nodejsPkgDir, "objectiveai_wasm_js.d.ts"),
  "utf-8"
);
writeFileSync(path.join(outDir, "loader.d.ts"), dtsContent);
console.log("✓ Created loader.d.ts");

console.log("\n✅ WASM installation complete (universal base64-embedded loader)");
