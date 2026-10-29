// The sea and the land, drawn once.

import * as Phaser from "phaser";
import type { WorldMap } from "../world/map";
import { cellNoise, valueNoise } from "../world/random";
import { FOAM, FOREST, GRASS, OCEAN, SAND, SHALLOWS, SHORE, TRUNK, shade } from "./palette";
import { CELL } from "./scale";

/** How wide the sand is along a coast, in pixels. */
const BEACH = 5;

/**
 * The terrain, as one graphics object beneath everything.
 *
 * The rules:
 *
 * 1. Every cell is filled with its terrain's colour, shaded a little
 *    by the world's noise — deep water darker, grass lighter and
 *    darker in patches.
 * 2. Every side of a land cell that faces water is sand, `BEACH`
 *    pixels deep, edged in dark on the water side; the water beside it
 *    carries a line of foam.
 * 3. Forest cells carry three small trees.
 */
export function drawTerrain(scene: Phaser.Scene, map: WorldMap): Phaser.GameObjects.Graphics {
  const g = scene.add.graphics();
  const { width, height, terrain } = map;
  const isLand = (x: number, y: number) => x >= 0 && y >= 0 && x < width && y < height && terrain[y * width + x] === "land";

  // Rule 1.
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const t = terrain[y * width + x];
      const patch = valueNoise(map.seed, x, y, 5) - 0.5;
      const grain = cellNoise(map.seed, x, y) - 0.5;
      const color = t === "land" ? shade(GRASS, patch * 0.25 + grain * 0.08) : t === "shallows" ? shade(SHALLOWS, grain * 0.06) : shade(OCEAN, -0.15 + patch * 0.2);
      g.fillStyle(color, 1);
      g.fillRect(x * CELL, y * CELL, CELL, CELL);
    }
  }

  // Rule 2.
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (!isLand(x, y)) continue;
      const px = x * CELL;
      const py = y * CELL;
      const sides = [
        { water: !isLand(x, y - 1), sand: [px, py, CELL, BEACH], edge: [px, py - 1, CELL, 2], foam: [px, py - 5, CELL, 2] },
        { water: !isLand(x, y + 1), sand: [px, py + CELL - BEACH, CELL, BEACH], edge: [px, py + CELL - 1, CELL, 2], foam: [px, py + CELL + 3, CELL, 2] },
        { water: !isLand(x - 1, y), sand: [px, py, BEACH, CELL], edge: [px - 1, py, 2, CELL], foam: [px - 5, py, 2, CELL] },
        { water: !isLand(x + 1, y), sand: [px + CELL - BEACH, py, BEACH, CELL], edge: [px + CELL - 1, py, 2, CELL], foam: [px + CELL + 3, py, 2, CELL] },
      ];
      for (const side of sides) {
        if (!side.water) continue;
        const [sx, sy, sw, sh] = side.sand;
        g.fillStyle(SAND, 1);
        g.fillRect(sx, sy, sw, sh);
        const [ex, ey, ew, eh] = side.edge;
        g.fillStyle(SHORE, 0.55);
        g.fillRect(ex, ey, ew, eh);
        const [fx, fy, fw, fh] = side.foam;
        g.fillStyle(FOAM, 0.35);
        g.fillRect(fx, fy, fw, fh);
      }
    }
  }

  // Rule 3.
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (!map.forest[y * width + x]) continue;
      const trees = [
        [0.3, 0.42],
        [0.68, 0.38],
        [0.5, 0.72],
      ];
      for (const [tx, ty] of trees) {
        const cx = (x + tx) * CELL;
        const cy = (y + ty) * CELL;
        g.fillStyle(TRUNK, 1);
        g.fillRect(cx - 1, cy + 4, 2, 4);
        g.fillStyle(shade(FOREST, (cellNoise(map.seed + 7, x * 3 + tx * 10, y) - 0.5) * 0.2), 1);
        g.fillTriangle(cx, cy - 7, cx - 6, cy + 5, cx + 6, cy + 5);
      }
    }
  }
  return g;
}
