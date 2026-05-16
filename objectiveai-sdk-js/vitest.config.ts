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
    // viewer subpath tests use jsdom; selected per-file via the
    // `// @vitest-environment jsdom` pragma at the top of those
    // test files (vitest 4 dropped environmentMatchGlobs). The
    // rest of the suite stays in the default node env.
  },
});
