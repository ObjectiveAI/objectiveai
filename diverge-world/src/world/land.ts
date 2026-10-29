// An island's land: what must be land, and the coast grown around it.

import { cellsUnder, type Point } from "./geometry";
import { valueNoise } from "./random";

/**
 * The land of one island, cell by cell, row by row, in a box `width`
 * by `height` whose border is never land.
 *
 * The rules:
 *
 * 1. Every cell within one cell of an entity is land, its eight
 *    neighbours included, so every entity stands on empty land.
 * 2. Every cell a line passes over, or passes near, is land, so the
 *    lines of an island run over it.
 * 3. Water that land closes in — a lake — is land.
 * 4. The coast then grows: a cell one cell from the land of rules 1 to
 *    3 is land where the island's noise is high enough, and a cell two
 *    cells from it where the noise is higher still.
 * 5. Water with five or more of its eight neighbours land becomes land,
 *    twice over, and lakes are filled again. Rules 4 and 5 only ever
 *    add land.
 */
export function land(width: number, height: number, entities: Point[], lines: [Point, Point][], seed: number): Uint8Array {
  const cells = new Uint8Array(width * height);
  const inside = (x: number, y: number) => x > 0 && y > 0 && x < width - 1 && y < height - 1;
  const set = (x: number, y: number) => {
    if (inside(x, y)) cells[y * width + x] = 1;
  };
  // Rule 1.
  for (const e of entities) {
    for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) set(e.x + dx, e.y + dy);
  }
  // Rule 2: from centre to centre.
  for (const [a, b] of lines) {
    for (const c of cellsUnder({ x: a.x + 0.5, y: a.y + 0.5 }, { x: b.x + 0.5, y: b.y + 0.5 }, 0.3)) set(c.x, c.y);
  }
  fillLakes(cells, width, height);

  // Rule 4: distance, in steps of eight neighbours, from the land so far.
  const distance = new Int32Array(width * height).fill(-1);
  let frontier: number[] = [];
  cells.forEach((c, i) => {
    if (c === 1) {
      distance[i] = 0;
      frontier.push(i);
    }
  });
  for (let d = 1; d <= 2; d++) {
    const next: number[] = [];
    for (const i of frontier) {
      const x = i % width;
      const y = Math.floor(i / width);
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          const nx = x + dx;
          const ny = y + dy;
          if (nx < 0 || ny < 0 || nx >= width || ny >= height) continue;
          const j = ny * width + nx;
          if (distance[j] >= 0) continue;
          distance[j] = d;
          next.push(j);
        }
      }
    }
    frontier = next;
  }
  for (let i = 0; i < cells.length; i++) {
    const x = i % width;
    const y = Math.floor(i / width);
    const noise = valueNoise(seed, x, y, 3);
    if ((distance[i] === 1 && noise > 0.35) || (distance[i] === 2 && noise > 0.7)) set(x, y);
  }

  // Rule 5.
  for (let pass = 0; pass < 2; pass++) {
    const grown: number[] = [];
    for (let y = 1; y < height - 1; y++) {
      for (let x = 1; x < width - 1; x++) {
        if (cells[y * width + x] === 1) continue;
        let around = 0;
        for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) around += cells[(y + dy) * width + x + dx];
        if (around >= 5) grown.push(y * width + x);
      }
    }
    for (const i of grown) cells[i] = 1;
  }
  fillLakes(cells, width, height);
  return cells;
}

/** Make land of every water cell the box's border cannot reach through water, four ways. */
function fillLakes(cells: Uint8Array, width: number, height: number): void {
  const open = new Uint8Array(width * height);
  const queue: number[] = [];
  for (let x = 0; x < width; x++) queue.push(x, (height - 1) * width + x);
  for (let y = 0; y < height; y++) queue.push(y * width, y * width + width - 1);
  while (queue.length > 0) {
    const i = queue.pop()!;
    if (open[i] === 1 || cells[i] === 1) continue;
    open[i] = 1;
    const x = i % width;
    const y = Math.floor(i / width);
    if (x > 0) queue.push(i - 1);
    if (x < width - 1) queue.push(i + 1);
    if (y > 0) queue.push(i - width);
    if (y < height - 1) queue.push(i + width);
  }
  for (let i = 0; i < cells.length; i++) if (open[i] === 0) cells[i] = 1;
}
