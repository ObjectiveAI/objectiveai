import { defineConfig, Options } from "tsup";
import path from "path";
// Base configuration shared between formats
const baseConfig: Options = {
  entry: ["src/index.ts", "src/viewer/index.ts"],
  dts: true,
  outDir: "dist",
  treeshake: true,
  target: "es2020",
  esbuildOptions(options) {
    options.alias = {
      src: path.resolve(__dirname, "src"),
    };
  },
};

export default defineConfig([
  // ESM build
  {
    ...baseConfig,
    format: ["esm"],
    outExtension: () => ({ js: ".js" }),
    clean: true,
  },
  // CJS build
  {
    ...baseConfig,
    format: ["cjs"],
    outExtension: () => ({ js: ".cjs" }),
    clean: false,
    dts: false,
    // Copy wasm loader to dist after build
    async onSuccess() {
      const { cpSync } = await import("fs");
      const srcWasm = path.resolve(__dirname, "src", "wasm");
      const distWasm = path.resolve(__dirname, "dist", "wasm");
      cpSync(srcWasm, distWasm, { recursive: true });
      console.log("✓ Copied wasm files to dist/wasm/");
    },
  },
]);
