import { describe, expect, it, vi } from "vitest";
import { persistentRuns } from "./useListener";
import type { CliCommandListenerExecution } from "@objectiveai/sdk";

type Runs = AsyncIterableIterator<CliCommandListenerExecution>;

/** A controllable fake for a daemonRuns() subscription. */
function fakeSubscription() {
  const queue: unknown[] = [];
  let ended = false;
  let wake: (() => void) | null = null;
  const iterator: Runs = {
    next: async () => {
      for (;;) {
        if (queue.length > 0) {
          return { value: queue.shift() as CliCommandListenerExecution, done: false };
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
      return iterator;
    },
  };
  return {
    iterator,
    push(run: unknown) {
      queue.push(run);
      wake?.();
    },
    get ended() {
      return ended;
    },
  };
}

describe("persistentRuns", () => {
  it("subscribes lazily on first next() and yields the inner's runs", async () => {
    const subs: ReturnType<typeof fakeSubscription>[] = [];
    const subscribe = vi.fn(() => {
      const s = fakeSubscription();
      subs.push(s);
      return s.iterator;
    });
    const stream = persistentRuns(subscribe);
    expect(subscribe).not.toHaveBeenCalled();

    const first = stream.next();
    expect(subscribe).toHaveBeenCalledTimes(1);
    subs[0].push({ run: 1 });
    expect((await first).value).toEqual({ run: 1 });
  });

  it("keeps one identity while resubscribing after return()", async () => {
    const subs: ReturnType<typeof fakeSubscription>[] = [];
    const subscribe = vi.fn(() => {
      const s = fakeSubscription();
      subs.push(s);
      return s.iterator;
    });
    const stream = persistentRuns(subscribe);
    expect(stream[Symbol.asyncIterator]()).toBe(stream);

    // First generation.
    const first = stream.next();
    subs[0].push({ gen: 1 });
    await first;

    // Detach (unmount cleanup): the live inner ends…
    await stream.return?.(undefined);
    expect(subs[0].ended).toBe(true);

    // …and the SAME facade resubscribes on the next use (remount).
    const second = stream.next();
    expect(subscribe).toHaveBeenCalledTimes(2);
    subs[1].push({ gen: 2 });
    expect((await second).value).toEqual({ gen: 2 });
  });

  it("resolves a parked consumer with done when return()'d", async () => {
    const subscribe = () => fakeSubscription().iterator;
    // Park a consumer with nothing queued, then detach out from
    // under it — the StrictMode fake-unmount shape.
    const stream = persistentRuns(() => {
      const s = fakeSubscription();
      return s.iterator;
    });
    void subscribe;
    const parked = stream.next();
    await stream.return?.(undefined);
    expect((await parked).done).toBe(true);
  });
});
