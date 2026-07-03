import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  resolve: {
    alias: {
      src: path.resolve(__dirname, "src"),
    },
  },
  test: {
    include: ["src/**/*.test.ts"],
    testTimeout: 0,
    // DOM-dependent tests (plugin-bridge routing) use jsdom, selected
    // per-file via the `// @vitest-environment jsdom` pragma at the
    // top of those test files (vitest 4 dropped
    // environmentMatchGlobs). Anything else stays in the default
    // node env.
  },
});
