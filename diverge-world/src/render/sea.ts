// The sea's pattern: one repeat of the hex grid, drawn once into a
// texture that tiles without end.

import * as Phaser from "phaser";
import { corners } from "../world/hex";
import { INK, SEA } from "./palette";
import { SIZE, WIDTH } from "./scale";

/** The texture's key. */
export const SEA_KEY = "sea";

/** Hex columns in one repeat. */
const COLUMNS = 11;
/** The texture's size: `COLUMNS` hexes wide, rounded to whole pixels, and two rows high, the grid's own period. */
export const SEA_WIDTH = Math.round(COLUMNS * WIDTH);
export const SEA_HEIGHT = 3 * SIZE;
/**
 * How much the texture squeezes the grid across, so `COLUMNS` hexes fill
 * a whole number of pixels: a texture pixel is this many world pixels'
 * worth across. The tiling undoes it.
 */
export const SQUEEZE = SEA_WIDTH / (COLUMNS * WIDTH);

/**
 * Make the sea's texture.
 *
 * The rules:
 *
 * 1. Its hexes are the map's hexes: texture pixel `(x, y)` is world pixel
 *    `(x / SQUEEZE, y)`, so a hex's centre is where `scale.pixel` puts
 *    the map's — row 0 at `SIZE` down, column 0 half a hex in, odd rows
 *    half a hex further.
 * 2. Every hex is filled with the sea's colour and outlined faintly in
 *    ink, as the land's hexes are.
 * 3. Hexes that run over an edge are drawn there too, so the pattern
 *    meets itself across every edge.
 */
export function makeSea(scene: Phaser.Scene): void {
  const g = scene.make.graphics({}, false);
  g.fillStyle(SEA, 1);
  g.fillRect(0, 0, SEA_WIDTH, SEA_HEIGHT);
  const outline = corners(SIZE);
  // Rule 3: one row above and one below, one column either side.
  for (let row = -1; row <= 2; row++) {
    for (let col = -1; col <= COLUMNS; col++) {
      const cx = (WIDTH * (col + (row & 1) / 2) + WIDTH / 2) * SQUEEZE;
      const cy = 1.5 * SIZE * row + SIZE;
      const shape = outline.map((p) => new Phaser.Math.Vector2(cx + p.x * SQUEEZE, cy + p.y));
      // Rule 2.
      g.lineStyle(1, INK, 0.18);
      g.strokePoints(shape, true);
    }
  }
  g.generateTexture(SEA_KEY, SEA_WIDTH, SEA_HEIGHT);
  g.destroy();
}
