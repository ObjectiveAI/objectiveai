import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  test: {
    environment: "jsdom",
    setupFiles: ["./vitest.setup.ts"],
    include: ["**/*.test.{ts,tsx}"],
    root: path.resolve(__dirname),
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "."),
    },
  },
});
