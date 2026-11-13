// The theme: its five colours and every role mixed from them
// (diverge-theme/theme.css, shared with the viewer), read off the page as colours the
// canvas and xterm can take — a role is often a `color-mix`, which only
// the page itself can resolve.

/** A role's resolved colour, by its name without the dashes, once read. */
const seen = new Map<string, [number, number, number]>();

/** The element the page resolves roles on: out of view, never shown. */
let probe: HTMLElement | null = null;

/** A computed colour's red, green and blue, 0–255: `rgb(…)`, `rgba(…)`, or `color(srgb …)`. */
function channels(computed: string): [number, number, number] {
  const srgb = computed.match(/color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)/);
  if (srgb) return [Number(srgb[1]), Number(srgb[2]), Number(srgb[3])].map((c) => Math.round(c * 255)) as [number, number, number];
  const rgb = computed.match(/rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)/);
  if (rgb) return [Number(rgb[1]), Number(rgb[2]), Number(rgb[3])].map((c) => Math.round(c)) as [number, number, number];
  return [0, 0, 0];
}

/**
 * A role's colour as red, green and blue.
 *
 * The rules:
 *
 * 1. A role is named as the theme names it, without its dashes: `bg`,
 *    `surface-2`, `line-strong`, `text-3`, `accent`, `here`.
 * 2. It is resolved by the page: the probe's colour set to the role,
 *    the computed colour read back, so a mix comes out as its colour.
 * 3. Each role is read once; the theme does not change while the page
 *    is open.
 */
function rgb(role: string): [number, number, number] {
  const known = seen.get(role);
  if (known) return known;
  // Rule 2.
  if (!probe) {
    probe = document.createElement("span");
    probe.style.display = "none";
    document.body.append(probe);
  }
  probe.style.color = `var(--${role})`;
  const value = channels(getComputedStyle(probe).color);
  // Rule 3.
  seen.set(role, value);
  return value;
}

/** A role's colour as a number, as Phaser draws with. */
export function int(role: string): number {
  const [r, g, b] = rgb(role);
  return (r << 16) | (g << 8) | b;
}

/** A role's colour as CSS, for a canvas text or xterm. */
export function css(role: string): string {
  const [r, g, b] = rgb(role);
  return `rgb(${r}, ${g}, ${b})`;
}
