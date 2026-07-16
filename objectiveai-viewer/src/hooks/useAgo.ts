import { useEffect, useState } from "react";
import { formatAgo, msUntilAgoChange } from "../lib/formatAgo";

/**
 * The live version of [`formatAgo`]: returns the relative string
 * ("just now", "3 minutes ago", …) as state, re-rendering exactly
 * when the string can change. The internal sleep is value-adaptive —
 * its duration comes from the unit of the CURRENT value: "just now"
 * sleeps to the 60s flip, minutes sleep to the next minute boundary,
 * hours to the next hour, day-and-older to the next day (which also
 * covers the week/month/year flips). Unparsable input (including
 * `""`) renders as `""` and never schedules anything.
 */
export function useAgo(value: string): string {
  const [text, setText] = useState(() => formatAgo(value));

  useEffect(() => {
    // A changed `value` recomputes immediately; the same effect then
    // owns the chain of boundary-aligned sleeps for that value.
    setText(formatAgo(value));
    let timer: ReturnType<typeof setTimeout> | null = null;
    let cancelled = false;
    const schedule = () => {
      const delay = msUntilAgoChange(value);
      if (!Number.isFinite(delay)) return;
      timer = setTimeout(() => {
        if (cancelled) return;
        setText(formatAgo(value));
        schedule();
      }, delay);
    };
    schedule();
    return () => {
      cancelled = true;
      if (timer !== null) clearTimeout(timer);
    };
  }, [value]);

  return text;
}
