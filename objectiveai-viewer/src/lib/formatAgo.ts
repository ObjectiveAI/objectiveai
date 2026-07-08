/** RFC3339 → a locale-aware relative string ("2 days ago", "3
 * minutes ago") via the built-in Intl.RelativeTimeFormat, using the
 * largest whole unit that fits. Empty string when unparsable. */
export function formatAgo(value: string): string {
  const then = new Date(value).getTime();
  if (Number.isNaN(then)) return "";
  const seconds = Math.max(0, Math.floor((Date.now() - then) / 1000));
  const format = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });
  const units: [Intl.RelativeTimeFormatUnit, number][] = [
    ["year", 31_557_600],
    ["month", 2_629_800],
    ["week", 604_800],
    ["day", 86_400],
    ["hour", 3_600],
    ["minute", 60],
    ["second", 1],
  ];
  for (const [unit, size] of units) {
    if (seconds >= size || unit === "second") {
      return format.format(-Math.floor(seconds / size), unit);
    }
  }
  return format.format(0, "second");
}
