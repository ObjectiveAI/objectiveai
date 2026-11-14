// The screen's pixel density: how many of the screen's pixels make one of
// the page's. The game draws in the screen's own pixels (`main.ts`), so
// a size meant on screen — a name, a grid line, a press's slack, the
// outliner's bounds — is multiplied by it.

let ratio = Math.max(1, window.devicePixelRatio || 1);
const told = new Set<(next: number, was: number) => void>();

/** The density now. */
export const density = (): number => ratio;

/** Be told each time the density changes; returns how to stop. */
export function onDensity(heard: (next: number, was: number) => void): () => void {
  told.add(heard);
  return () => told.delete(heard);
}

/**
 * Watch the density: the window moved to a screen of another density,
 * or the page zoomed. Each change tells every listener once, after the
 * new density is the one `density` says.
 */
export function watchDensity(): void {
  const watch = () =>
    matchMedia(`(resolution: ${ratio}dppx)`).addEventListener(
      "change",
      () => {
        const was = ratio;
        ratio = Math.max(1, window.devicePixelRatio || 1);
        if (ratio !== was) for (const heard of told) heard(ratio, was);
        watch();
      },
      { once: true },
    );
  watch();
}
