import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    // Each test spawns subprocesses; parallel runs across files would
    // race for free ports and bloat memory. Run files serially; tests
    // *within* a file can still run in parallel since each owns its own
    // rig and ports.
    fileParallelism: false,
    testTimeout: 30_000,
    hookTimeout: 30_000,
  },
});
