// How big the world is drawn, and where a hex lands on it.

import type { Point } from "../world/geometry";
import { centre, type Hex } from "../world/hex";

/** Pixels from a hex's centre to its corners, at a zoom of 1. */
export const SIZE = 21;

/** A hex's width in pixels: corner-to-corner height is `2 * SIZE`. */
export const WIDTH = Math.sqrt(3) * SIZE;

/** A hex's centre in pixels, the stored grid's first hex just inside the top-left. */
export function pixel(h: Hex): Point {
  const c = centre(h);
  return { x: c.x * SIZE + WIDTH / 2, y: c.y * SIZE + SIZE };
}

/** The size in pixels of a world `columns` by `rows` of the stored grid. */
export const extent = (columns: number, rows: number) => ({ width: WIDTH * (columns + 0.5), height: 1.5 * SIZE * (rows - 1) + 2 * SIZE });
