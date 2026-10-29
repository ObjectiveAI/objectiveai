// When something happened, as "3 minutes ago", kept true as time goes.

const FORMAT = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

/** How often every shown time is redrawn. */
const EVERY = 30_000;

/** `at` against now, in the largest unit that fits: under a minute is "just now". */
export function ago(at: Date, now = Date.now()): string {
  const seconds = Math.round((at.getTime() - now) / 1000);
  const abs = Math.abs(seconds);
  if (abs < 60) return "just now";
  if (abs < 3600) return FORMAT.format(Math.round(seconds / 60), "minute");
  if (abs < 86400) return FORMAT.format(Math.round(seconds / 3600), "hour");
  return FORMAT.format(Math.round(seconds / 86400), "day");
}

let timer: number | null = null;

/**
 * An element showing when `created` was, which stays true.
 *
 * One timer for the whole page redraws every element marked
 * `data-at` every `EVERY` milliseconds; it starts with the first.
 */
export function agoElement(created: string): HTMLElement {
  const el = document.createElement("time");
  el.className = "ago";
  el.dataset.at = created;
  el.dateTime = created;
  el.title = new Date(created).toLocaleString();
  el.textContent = ago(new Date(created));
  if (timer === null) {
    timer = window.setInterval(() => {
      for (const t of document.querySelectorAll<HTMLElement>("[data-at]")) {
        t.textContent = ago(new Date(t.dataset.at ?? ""));
      }
    }, EVERY);
  }
  return el;
}
