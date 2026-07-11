/** RFC3339 → a locale-aware relative string ("2 days ago", "3
 * minutes ago") via the built-in Intl.RelativeTimeFormat, using the
 * largest whole unit that fits. Anything under a minute is
 * "just now" (no seconds granularity — see [`msUntilAgoChange`]).
 * Empty string when unparsable. */
export function formatAgo(value: string): string {
  const then = new Date(value).getTime();
  if (Number.isNaN(then)) return "";
  const seconds = Math.max(0, Math.floor((Date.now() - then) / 1000));
  if (seconds < 60) return "just now";
  const format = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });
  const units: [Intl.RelativeTimeFormatUnit, number][] = [
    ["year", 31_557_600],
    ["month", 2_629_800],
    ["week", 604_800],
    ["day", 86_400],
    ["hour", 3_600],
    ["minute", 60],
  ];
  for (const [unit, size] of units) {
    if (seconds >= size) {
      return format.format(-Math.floor(seconds / size), unit);
    }
  }
  return format.format(-1, "minute");
}

/** Milliseconds until [`formatAgo`]'s output for `value` can next
 * change — the tick rate derives from the CURRENT value's unit:
 * "just now" flips at the 60s mark, minutes tick per minute, hours
 * per hour, and everything day-or-older per day (which also covers
 * the week/month/year flips — a day boundary is a superset of all
 * of them). Scheduled to the exact unit boundary rather than a
 * fixed phase, floored at 1s. `Infinity` when unparsable (nothing
 * will ever change). */
export function msUntilAgoChange(value: string): number {
  const then = new Date(value).getTime();
  if (Number.isNaN(then)) return Infinity;
  const elapsed = Math.max(0, (Date.now() - then) / 1000);
  let delay: number;
  if (elapsed < 60) {
    delay = 60 - elapsed;
  } else if (elapsed < 3_600) {
    delay = 60 - (elapsed % 60);
  } else if (elapsed < 86_400) {
    delay = 3_600 - (elapsed % 3_600);
  } else {
    delay = 86_400 - (elapsed % 86_400);
  }
  return Math.max(1_000, Math.ceil(delay * 1000));
}
