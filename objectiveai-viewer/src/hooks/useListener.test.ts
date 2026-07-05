// @vitest-environment jsdom

import { describe, expect, it, beforeEach, vi } from "vitest";
import { act, createElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { CliCommandListenerExecution } from "@objectiveai/sdk";

/**
 * Tests for useListener, exercised through a real React mount (the
 * persistentRuns facade is internal). The daemon-listener module is
 * mocked with a controllable fake subscription factory.
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

type Stream = AsyncIterableIterator<CliCommandListenerExecution>;

const harness = vi.hoisted(() => {
  type FakeSub = {
    iterator: unknown;
    push(run: unknown): void;
    ended: boolean;
  };
  const h = {
    subs: [] as FakeSub[],
    subscribeCalls: 0,
    reset() {
      h.subs.length = 0;
      h.subscribeCalls = 0;
    },
    newSub(): FakeSub {
      const queue: unknown[] = [];
      let ended = false;
      let wake: (() => void) | null = null;
      const sub: FakeSub = {
        get ended() {
          return ended;
        },
        push(run: unknown) {
          queue.push(run);
          wake?.();
        },
        iterator: {
          next: async () => {
            for (;;) {
              if (queue.length > 0) {
                return { value: queue.shift(), done: false };
              }
              if (ended) return { value: undefined, done: true };
              await new Promise<void>((r) => {
                wake = r;
              });
              wake = null;
            }
          },
          return: async () => {
            ended = true;
            wake?.();
            return { value: undefined, done: true };
          },
          throw: async (e?: unknown) => {
            ended = true;
            wake?.();
            throw e;
          },
          [Symbol.asyncIterator]() {
            return sub.iterator;
          },
        },
      };
      h.subs.push(sub);
      return sub;
    },
  };
  return h;
});

vi.mock("../daemon-listener", () => ({
  daemonRuns: () => {
    harness.subscribeCalls += 1;
    return harness.newSub().iterator;
  },
}));

import { useListener } from "./useListener";

/** Mount a probe component that calls the hook; return controls. */
function mountProbe() {
  const streams: Stream[] = [];
  function Probe() {
    streams.push(useListener());
    return null;
  }
  const container = document.createElement("div");
  const root: Root = createRoot(container);
  act(() => {
    root.render(createElement(Probe));
  });
  return {
    streams,
    rerender: () =>
      act(() => {
        root.render(createElement(Probe));
      }),
    unmount: () =>
      act(() => {
        root.unmount();
      }),
  };
}

describe("useListener", () => {
  beforeEach(() => {
    harness.reset();
  });

  it("returns the same stream on every re-render and subscribes lazily", () => {
    const probe = mountProbe();
    probe.rerender();
    probe.rerender();
    expect(probe.streams.length).toBeGreaterThanOrEqual(3);
    for (const s of probe.streams) {
      expect(s).toBe(probe.streams[0]);
    }
    // No consumer has pulled yet — no subscription exists.
    expect(harness.subscribeCalls).toBe(0);
    probe.unmount();
  });

  it("yields the runs the singleton fans out", async () => {
    const probe = mountProbe();
    const stream = probe.streams[0];

    const first = stream.next();
    expect(harness.subscribeCalls).toBe(1);
    harness.subs[harness.subs.length - 1].push({ run: 1 });
    expect((await first).value).toEqual({ run: 1 });
    probe.unmount();
  });

  it("ends parked consumers on unmount", async () => {
    const probe = mountProbe();
    const stream = probe.streams[0];

    const parked = stream.next();
    probe.unmount();
    expect((await parked).done).toBe(true);
    // The live subscription underneath was released.
    expect(harness.subs[harness.subs.length - 1].ended).toBe(true);
  });

  it("resubscribes through the same stream after a detach (StrictMode remount shape)", async () => {
    const probe = mountProbe();
    const stream = probe.streams[0];

    // Generation 1.
    const first = stream.next();
    harness.subs[harness.subs.length - 1].push({ gen: 1 });
    await first;

    // Detach out from under the consumer (what the hook's cleanup
    // does on StrictMode's fake unmount)…
    await stream.return?.(undefined);

    // …and the SAME stream object comes back to life on next use.
    const second = stream.next();
    expect(harness.subscribeCalls).toBe(2);
    harness.subs[harness.subs.length - 1].push({ gen: 2 });
    expect((await second).value).toEqual({ gen: 2 });
    probe.unmount();
  });
});
