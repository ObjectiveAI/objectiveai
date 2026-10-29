// The sea and the land, drawn once, hex by hex.

import * as Phaser from "phaser";
import { corners, fromOffset, STEPS } from "../world/hex";
import { isLand, type WorldMap } from "../world/map";
import { GRASS, INK, SAND, SEA, SHORE } from "./palette";
import { pixel, SIZE } from "./scale";

/** How far in from a coast the sand reaches, as a share of the way to the centre. */
const BEACH = 0.16;

/**
 * The terrain, as one graphics object beneath everything.
 *
 * The rules:
 *
 * 1. Every hex of the stored grid is filled: land in one green, sea in
 *    one blue. Nothing else grows.
 * 2. Every hex is outlined faintly in ink, so the grid shows.
 * 3. Every side of a land hex that faces sea is sand, a band along that
 *    side, edged in dark on the sea side.
 */
export function drawTerrain(scene: Phaser.Scene, map: WorldMap): Phaser.GameObjects.Graphics {
  const g = scene.add.graphics();
  const outer = corners(SIZE);
  const inner = corners(SIZE * (1 - BEACH));
  for (let row = 0; row < map.rows; row++) {
    for (let col = 0; col < map.columns; col++) {
      const h = fromOffset({ col, row });
      const c = pixel(h);
      const shape = outer.map((p) => new Phaser.Math.Vector2(c.x + p.x, c.y + p.y));
      const landed = map.land[row * map.columns + col] === 1;
      // Rule 1.
      g.fillStyle(landed ? GRASS : SEA, 1);
      g.fillPoints(shape, true);
      // Rule 3.
      if (landed) {
        STEPS.forEach((step, k) => {
          if (isLand(map, { q: h.q + step.q, r: h.r + step.r })) return;
          const a = outer[k];
          const b = outer[(k + 1) % 6];
          const band = [a, b, inner[(k + 1) % 6], inner[k]].map((p) => new Phaser.Math.Vector2(c.x + p.x, c.y + p.y));
          g.fillStyle(SAND, 1);
          g.fillPoints(band, true);
          g.lineStyle(1.5, SHORE, 0.7);
          g.lineBetween(c.x + a.x, c.y + a.y, c.x + b.x, c.y + b.y);
        });
      }
      // Rule 2.
      g.lineStyle(1, INK, 0.18);
      g.strokePoints(shape, true);
    }
  }
  return g;
}
