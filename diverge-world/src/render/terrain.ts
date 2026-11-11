// The land, drawn once, hex by hex; the sea is the sea scene's.

import * as Phaser from "phaser";
import { corners, fromOffset, STEPS } from "../world/hex";
import { isLand, type WorldMap } from "../world/map";
import { GRASS, INK, SAND, SHORE } from "./palette";
import { pixel, SIZE } from "./scale";

/** How far in from a coast the sand reaches, as a share of the way to the centre. */
const BEACH = 0.16;

/**
 * The terrain, as one graphics object beneath everything.
 *
 * The rules:
 *
 * 1. Every land hex is filled in one green. Nothing else grows, and no
 *    sea hex is drawn here: the sea scene's pattern is under them all.
 * 2. Every land hex is outlined faintly in ink, as the sea's are, so
 *    the grid shows.
 * 3. Every side of a land hex that faces sea is sand, a band along that
 *    side, edged in dark on the sea side.
 */
export function drawTerrain(scene: Phaser.Scene, map: WorldMap): Phaser.GameObjects.Graphics {
  const g = scene.add.graphics();
  const outer = corners(SIZE);
  const inner = corners(SIZE * (1 - BEACH));
  for (let row = 0; row < map.rows; row++) {
    for (let col = 0; col < map.columns; col++) {
      if (map.land[row * map.columns + col] !== 1) continue;
      const h = fromOffset({ col, row });
      const c = pixel(h);
      const shape = outer.map((p) => new Phaser.Math.Vector2(c.x + p.x, c.y + p.y));
      // Rule 1.
      g.fillStyle(GRASS, 1);
      g.fillPoints(shape, true);
      // Rule 3.
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
      // Rule 2.
      g.lineStyle(1, INK, 0.18);
      g.strokePoints(shape, true);
    }
  }
  return g;
}
