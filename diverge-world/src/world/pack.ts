// Where each island goes in the sea.

import { type Hex, toOffset } from "./hex";

/** Columns or rows of sea between two islands' boxes, and around them all. */
const GAP = 3;

/** One island's place: the step that takes its own hexes to the world's. */
export type Move = Hex;

/**
 * The step for every island, in the order given, and the size of the
 * whole sea, in columns and rows of the stored grid.
 *
 * The rules:
 *
 * 1. Each island is boxed by the columns and rows its land takes in
 *    the stored grid.
 * 2. Boxes are laid tallest first (then widest, then first given), left
 *    to right on shelves, a shelf as high as its tallest box.
 * 3. A shelf is about as wide as the whole is high: as wide as the
 *    square root of the boxes' area, or the widest box if wider; a box
 *    that would run past it starts the next shelf.
 * 4. Every island moves down by an even number of rows, so the odd rows
 *    it had stay odd and its shape is kept hex for hex.
 * 5. `GAP` columns or rows of sea lie between any two boxes and around
 *    them all, so no two islands' land comes within two hexes of the
 *    other's.
 */
export function pack(islands: Hex[][]): { moves: Move[]; columns: number; rows: number } {
  // Rule 1, and rule 4's first half: each island stepped so its box starts at column 0 and at row 0 or 1.
  const boxes = islands.map((land) => {
    const rows = land.map((h) => h.r);
    const minRow = Math.min(...rows);
    const lift = minRow - (minRow & 1);
    const cols = land.map((h) => toOffset({ q: h.q, r: h.r - lift }).col);
    const minCol = Math.min(...cols);
    // `cols` are already the columns after the step `(0, -lift)`; `(-minCol, 0)` then starts them at 0.
    const start: Hex = { q: -minCol, r: -lift };
    const width = Math.max(...cols) - minCol + 1;
    const height = Math.max(...rows) - lift + 1;
    return { start, width, height };
  });

  // Rules 2, 3 and 5.
  const order = boxes.map((_, i) => i).sort((i, j) => boxes[j].height - boxes[i].height || boxes[j].width - boxes[i].width || i - j);
  const area = boxes.reduce((sum, b) => sum + (b.width + GAP) * (b.height + GAP), 0);
  const target = Math.max(...boxes.map((b) => b.width), Math.ceil(Math.sqrt(area)));
  const moves: Move[] = boxes.map(() => ({ q: 0, r: 0 }));
  const even = (n: number) => n + (n & 1);
  let x = GAP;
  let y = even(GAP);
  let shelf = 0;
  let columns = 0;
  for (const i of order) {
    const box = boxes[i];
    if (x > GAP && x + box.width > GAP + target) {
      x = GAP;
      y = even(y + shelf + GAP);
      shelf = 0;
    }
    // Rule 4's second half: `y` is even, so the step down is. The step `(dq, dr)` with `dr` even
    // moves the stored grid by `(dq + dr / 2, dr)`, so `(x - y / 2, y)` moves it by `(x, y)`.
    moves[i] = { q: box.start.q + x - y / 2, r: box.start.r + y };
    x += box.width + GAP;
    columns = Math.max(columns, x);
    shelf = Math.max(shelf, box.height);
  }
  return { moves, columns, rows: islands.length === 0 ? 0 : y + shelf + GAP };
}
