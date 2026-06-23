import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // The four http.test.ts suites; they import the built @objectiveai/sdk
    // by package name (resolved to dist via the pnpm workspace link) and
    // skip themselves when OBJECTIVEAI_ADDRESS is unset.
    include: ["src/**/*.test.ts"],
    testTimeout: 0,
  },
});
