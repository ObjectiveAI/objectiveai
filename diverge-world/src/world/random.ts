// Chance that repeats: the same seed draws the same world, so one
// daemon state always looks the same.

/** A string's 32-bit FNV-1a hash: the seed a set of keys is drawn from. */
export function hash(text: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

/** A generator of numbers in `[0, 1)` from `seed` (mulberry32). */
export function generator(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** One cell's own number in `[0, 1)`, the same for the same seed and cell. */
export function cellNoise(seed: number, x: number, y: number): number {
  let h = Math.imul(x, 374761393) + Math.imul(y, 668265263) + Math.imul(seed, 2246822519);
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

/** Smooth noise in `[0, 1)`: the cell numbers of a grid `scale` cells wide, blended between. */
export function valueNoise(seed: number, x: number, y: number, scale: number): number {
  const gx = x / scale;
  const gy = y / scale;
  const x0 = Math.floor(gx);
  const y0 = Math.floor(gy);
  const fx = smooth(gx - x0);
  const fy = smooth(gy - y0);
  const top = lerp(cellNoise(seed, x0, y0), cellNoise(seed, x0 + 1, y0), fx);
  const bottom = lerp(cellNoise(seed, x0, y0 + 1), cellNoise(seed, x0 + 1, y0 + 1), fx);
  return lerp(top, bottom, fy);
}

const smooth = (t: number) => t * t * (3 - 2 * t);
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
