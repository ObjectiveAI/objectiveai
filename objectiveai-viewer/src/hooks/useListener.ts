import { useEffect, useRef } from "react";
import type { CliCommandListenerExecution } from "@objectiveai/sdk";
import { daemonRuns } from "../daemon-listener";

type Runs = AsyncIterableIterator<CliCommandListenerExecution>;

/**
 * A persistent handle on the app's singleton daemon listener: one
 * identity-stable run stream per component instance — the SAME object
 * on every re-render, never re-created, safe to close over or put in
 * effect deps.
 *
 * The returned stream is a thin facade over a `daemonRuns()`
 * subscription: `next()` lazily (re)subscribes when there is no live
 * subscription underneath, and the hook's unmount cleanup `return()`s
 * the live one (which also wakes any parked consumer with `done`).
 * That lazy re-attach is what makes React StrictMode's dev-mode
 * mount→unmount→remount harmless: the remounted effect's loop simply
 * subscribes afresh through the same facade. Live-only as always —
 * runs announced while detached (or before mount) are missed.
 *
 * ```tsx
 * const listener = useListener();
 * useEffect(() => {
 *   void (async () => {
 *     for await (const run of listener) { … }
 *   })();
 *   // No manual cleanup needed: the hook ends the loop on unmount.
 * }, [listener]);
 * ```
 */
export function useListener(): Runs {
  const ref = useRef<Runs | null>(null);
  ref.current ??= persistentRuns();

  useEffect(() => {
    const stream = ref.current;
    return () => {
      void stream?.return?.(undefined);
    };
  }, []);

  return ref.current;
}

/**
 * The facade behind [`useListener`] — exported for tests, which
 * inject a fake subscribe function. `subscribe` defaults to the
 * singleton's `daemonRuns`.
 */
export function persistentRuns(subscribe: () => Runs = daemonRuns): Runs {
  let inner: Runs | null = null;

  const facade: Runs = {
    next: (): Promise<IteratorResult<CliCommandListenerExecution>> => {
      inner ??= subscribe();
      return inner.next();
    },
    return: async (): Promise<IteratorResult<CliCommandListenerExecution>> => {
      const live = inner;
      inner = null;
      if (live) {
        await live.return?.(undefined);
      }
      return { value: undefined, done: true };
    },
    throw: async (
      e?: unknown,
    ): Promise<IteratorResult<CliCommandListenerExecution>> => {
      await facade.return?.(undefined);
      throw e;
    },
    [Symbol.asyncIterator]() {
      return facade;
    },
  };
  return facade;
}
