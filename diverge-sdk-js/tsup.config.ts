import { defineConfig } from "tsup";

// One ESM bundle. The package is `type: module` and nothing else: a
// CommonJS build would be a second copy of every module with its own
// identity, which is the dual-package hazard, and Node 22 requires an
// ES module from CommonJS without one. The declarations are emitted
// beside it by TypeScript itself (`tsconfig.build.json`), one `.d.ts`
// per source file: TypeScript 7 ships no compiler API for a bundler
// to drive.
export default defineConfig({
  entry: ["src/index.ts"],
  format: ["esm"],
  dts: false,
  target: "es2022",
  platform: "neutral",
  sourcemap: true,
  clean: true,
});
