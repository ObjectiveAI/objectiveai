import { useLayoutEffect, useRef } from "react";

/**
 * Bottom-tethered scrolling for top-aligned append-only lists (the
 * log panes): when the scroller sits at the bottom, new content keeps
 * it there; scrolling up releases the tether until the user returns
 * to the bottom. The conversation view gets this for free from its
 * col-reverse layout — these panes are deliberately normal-flow
 * (short content must sit at the TOP), so the tether is explicit.
 *
 * `dep` is whatever changes when content grows (the entries map).
 * Attach BOTH returns to the scroll container. The tether starts
 * armed — an empty container IS at the bottom — so the history pull
 * lands you on the newest entries.
 */
export function useBottomTether<T>(dep: T): {
  ref: React.RefObject<HTMLDivElement | null>;
  onScroll: () => void;
} {
  const ref = useRef<HTMLDivElement | null>(null);
  const pinned = useRef(true);
  const onScroll = () => {
    const el = ref.current;
    if (el === null) return;
    // ≤1px slack: fractional DPR scaling can leave scrollTop a hair
    // shy of the true bottom.
    pinned.current = el.scrollHeight - el.scrollTop - el.clientHeight < 2;
  };
  useLayoutEffect(() => {
    const el = ref.current;
    if (el !== null && pinned.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [dep]);
  return { ref, onScroll };
}
