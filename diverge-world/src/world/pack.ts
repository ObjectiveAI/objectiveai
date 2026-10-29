// Where each island goes in the sea.

import type { Point } from "./geometry";

/** An island's box, in cells. */
export type Box = { width: number; height: number };

/** Cells of water between two islands' boxes, and around them all, beyond the water each box keeps at its border. */
const GAP = 2;

/**
 * The top-left cell of every box, in the order given, and the size of
 * the whole sea.
 *
 * The rules:
 *
 * 1. Boxes are laid tallest first (then widest, then first given), left
 *    to right on shelves, a shelf as high as its tallest box.
 * 2. A shelf is about as wide as the whole is high: as wide as the
 *    square root of the boxes' area, or the widest box if wider; a
 *    box that would run past it starts the next shelf.
 * 3. `GAP` cells of water lie between any two boxes and around them
 *    all. Each box's own border is water too, so no two islands come
 *    within three cells of each other.
 */
export function pack(boxes: Box[]): { origins: Point[]; width: number; height: number } {
  const order = boxes.map((_, i) => i).sort((i, j) => boxes[j].height - boxes[i].height || boxes[j].width - boxes[i].width || i - j);
  const area = boxes.reduce((sum, b) => sum + (b.width + GAP) * (b.height + GAP), 0);
  const target = Math.max(...boxes.map((b) => b.width), Math.ceil(Math.sqrt(area)));
  const origins: Point[] = boxes.map(() => ({ x: 0, y: 0 }));
  let x = GAP;
  let y = GAP;
  let shelf = 0;
  let width = 0;
  for (const i of order) {
    const box = boxes[i];
    if (x > GAP && x + box.width > GAP + target) {
      x = GAP;
      y += shelf + GAP;
      shelf = 0;
    }
    origins[i] = { x, y };
    x += box.width + GAP;
    width = Math.max(width, x);
    shelf = Math.max(shelf, box.height);
  }
  return { origins, width, height: boxes.length === 0 ? 0 : y + shelf + GAP };
}
