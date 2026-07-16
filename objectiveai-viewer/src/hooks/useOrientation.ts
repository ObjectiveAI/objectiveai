import { useSyncExternalStore } from "react";

/** Which way the hierarchy tree DESCENDS. `vertical` is the historic
 * layout: tiers stack top-down, siblings flow left-to-right.
 * `horizontal` is the transpose (mobile-friendly — screens are
 * taller than wide): tiers stack left-to-right, siblings flow
 * top-down. */
export type Orientation = "vertical" | "horizontal";

// Module store, same shape as `lib/errors`: plain module state +
// subscriber set, consumed via useSyncExternalStore. Any component
// can read it with the hook; the footer toggle writes it.
let orientation: Orientation = "vertical";
const subscribers = new Set<() => void>();

function subscribe(callback: () => void): () => void {
  subscribers.add(callback);
  return () => {
    subscribers.delete(callback);
  };
}

export function toggleOrientation(): void {
  orientation = orientation === "vertical" ? "horizontal" : "vertical";
  for (const subscriber of [...subscribers]) {
    subscriber();
  }
}

/** The current tree orientation, live. */
export function useOrientation(): Orientation {
  return useSyncExternalStore(subscribe, () => orientation);
}
