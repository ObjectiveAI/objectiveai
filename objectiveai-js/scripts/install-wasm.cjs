const { spawnSync } = require("child_process");
const {
  mkdirSync,
  readFileSync,
  writeFileSync,
  rmSync,
  existsSync,
} = require("fs");
const path = require("path");

// Paths
const jsRoot = process.cwd(); // objectiveai-js
const repoRoot = path.resolve(jsRoot, ".."); // objectiveai
const wasmDir = path.join(repoRoot, "objectiveai-rs-wasm-js");
const outDir = path.join(jsRoot, "src", "wasm");
const wasmDistDir = path.join(wasmDir, "dist");

// If WASM dist is unavailable but output files already exist, skip (e.g. Docker build)
if (!existsSync(wasmDistDir) && existsSync(path.join(outDir, "loader.cjs"))) {
  console.log("✓ WASM loader already present, skipping (no WASM dist available)");
  process.exit(0);
}

// 1. Validate dist/ is up to date
const validateResult = spawnSync("bash", [path.join(wasmDir, "validate.sh")], {
  stdio: "inherit",
  shell: process.platform === "win32",
});

if (validateResult.status !== 0) {
  process.exit(validateResult.status ?? 1);
}

// Clean up old output files
if (existsSync(outDir)) {
  rmSync(outDir, { recursive: true });
}
mkdirSync(outDir, { recursive: true });

// 2. Read the generated files
const glueCode = readFileSync(
  path.join(wasmDistDir, "objectiveai_wasm_js.js"),
  "utf-8",
);
const wasmBinary = readFileSync(
  path.join(wasmDistDir, "objectiveai_wasm_js_bg.wasm"),
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
const wasm = exports.__wasm = new WebAssembly.Instance(wasmModule, __wbg_get_imports()).exports;

wasm.__wbindgen_start();`;

// Also need to handle the decodeText function which uses TextDecoder
// The nodejs target might use a Node.js-specific approach
let modifiedGlue = glueCode.replace(fsLoadPattern, universalLoaderCode);

// Check if there's a require('util') for TextDecoder - make it universal
if (modifiedGlue.includes("require('util')")) {
  // Replace Node.js TextDecoder with universal version
  modifiedGlue = modifiedGlue.replace(
    /const \{ TextDecoder \} = require\('util'\);/g,
    "const TextDecoder = typeof globalThis.TextDecoder !== 'undefined' ? globalThis.TextDecoder : require('util').TextDecoder;",
  );
}

// 4. Write the CJS version (the modified glue code is already CJS)
writeFileSync(path.join(outDir, "loader.cjs"), modifiedGlue);
console.log("✓ Created loader.cjs (universal CJS loader)");

// 5. Read type declarations and extract exported function names
const dtsContent = readFileSync(
  path.join(wasmDistDir, "objectiveai_wasm_js.d.ts"),
  "utf-8",
);

// Parse "export function foo(...)" declarations from the .d.ts file
const exportedFunctions = [...dtsContent.matchAll(/^export function (\w+)\(/gm)].map(m => m[1]);
console.log(`✓ Found ${exportedFunctions.length} exported WASM functions`);

// 6. Create ESM version by wrapping the CJS code in an IIFE
// This avoids symbol conflicts with the function declarations inside
const esmExports = exportedFunctions.map(fn => `export const ${fn} = _wasm.${fn};`).join("\n");
const esmLoader = `// Universal ESM loader with embedded base64 WASM
// Works in Node.js and browsers without bundler configuration

const _wasm = (() => {
  const exports = {};
  const module = { exports };
  ${modifiedGlue.replace(
    "imports['__wbindgen_placeholder__'] = module.exports;",
    "imports['__wbindgen_placeholder__'] = exports;",
  )}
  return exports;
})();

${esmExports}
`;

writeFileSync(path.join(outDir, "loader.js"), esmLoader);
console.log("✓ Created loader.js (universal ESM loader)");

// 7. Copy type declarations
writeFileSync(path.join(outDir, "loader.d.ts"), dtsContent);
console.log("✓ Created loader.d.ts");

console.log(
  "\n✅ WASM installation complete (universal base64-embedded loader)",
);
