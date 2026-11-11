// The links: a line with an arrow for every way something passed
// between two entities, drawn over the land and the sea.

import * as Phaser from "phaser";
import { exitPolygon, type Point } from "../world/geometry";
import { corners } from "../world/hex";
import type { WorldMap } from "../world/map";
import { CREAM, INK, KIND, shade } from "./palette";
import { pixel, SIZE } from "./scale";

/** How far either line of a two-way pair lies from the pair's middle, in pixels. */
const APART = 5;
/** How far past a hex's edge a line stops, in pixels. */
const CLEAR = 2;
/** An arrowhead's length and half its width, in pixels. */
const HEAD = 9;
const HALF = 5;

/** A hex's outline, `CLEAR` pixels out, about its centre. */
const OUTLINE = corners(SIZE + CLEAR);

/**
 * Every pair's lines, as one graphics object above the terrain and
 * below the entities.
 *
 * The rules:
 *
 * 1. A pair joined one way is one line, between the two hexes' centres.
 * 2. A pair joined both ways is two lines, `APART` pixels either side of
 *    the centres' line, each pointing its own way: from `a` to `b` on one
 *    side, from `b` to `a` on the other.
 * 3. Every line starts where it leaves its sender's hex and ends where
 *    it meets its receiver's, `CLEAR` pixels outside each, so it never
 *    covers an entity.
 * 4. Every line ends in an arrowhead, its tip at the receiver.
 * 5. Every line and head is cream over a wider stroke of ink, so it
 *    reads over grass and sea alike.
 * 6. A mount — an agent and the volume it mounts — is a dashed line in
 *    the volume's colour over ink, with no arrowhead, beneath the
 *    arrows.
 */
export function drawLinks(scene: Phaser.Scene, map: WorldMap): Phaser.GameObjects.Graphics {
  const g = scene.add.graphics().setDepth(1);
  const centreOf = (key: string): Point | null => {
    const at = map.at.get(key);
    return at ? pixel(at) : null;
  };
  const arrows: { from: Point; to: Point }[] = [];
  const mounts: { from: Point; to: Point }[] = [];
  for (const pair of map.pairs) {
    const a = centreOf(pair.a);
    const b = centreOf(pair.b);
    if (!a || !b) continue;
    if (pair.mount) {
      mounts.push(cut(a, b, 0));
      continue;
    }
    // Rules 1 and 2.
    const offset = pair.ab && pair.ba ? APART : 0;
    if (pair.ab) arrows.push(cut(a, b, offset));
    if (pair.ba) arrows.push(cut(b, a, offset));
  }
  // Rule 6, first, so the arrows lie over the mounts.
  for (const [width, color] of [[4, INK], [2, shade(KIND.volume, 0.35)]] as const) {
    g.lineStyle(width, color, 1);
    for (const { from, to } of mounts) dashed(g, from, to);
  }
  // Rule 5: all the ink first, then all the cream, so no line's ink covers another's cream.
  for (const [width, color, grow] of [[4, INK, 1.8], [2, CREAM, 0]] as const) {
    for (const { from, to } of arrows) {
      const d = unit(from, to);
      const n = { x: -d.y, y: d.x };
      const base = { x: to.x - d.x * HEAD, y: to.y - d.y * HEAD };
      g.lineStyle(width, color, 1);
      g.lineBetween(from.x, from.y, base.x + d.x, base.y + d.y);
      // Rule 4.
      g.fillStyle(color, 1);
      g.fillTriangle(
        to.x + d.x * grow,
        to.y + d.y * grow,
        base.x - d.x * grow + n.x * (HALF + grow),
        base.y - d.y * grow + n.y * (HALF + grow),
        base.x - d.x * grow - n.x * (HALF + grow),
        base.y - d.y * grow - n.y * (HALF + grow),
      );
    }
  }
  return g;
}

/** How long a mount's dash and its gap are, in pixels. */
const DASH = 6;
const GAP = 4;

/** A dashed line from `from` to `to`, in the graphics' line style. */
function dashed(g: Phaser.GameObjects.Graphics, from: Point, to: Point): void {
  const length = Math.hypot(to.x - from.x, to.y - from.y);
  const d = unit(from, to);
  for (let at = 0; at < length; at += DASH + GAP) {
    const end = Math.min(at + DASH, length);
    g.lineBetween(from.x + d.x * at, from.y + d.y * at, from.x + d.x * end, from.y + d.y * end);
  }
}

/** The unit direction from `a` to `b`. */
function unit(a: Point, b: Point): Point {
  const length = Math.hypot(b.x - a.x, b.y - a.y) || 1;
  return { x: (b.x - a.x) / length, y: (b.y - a.y) / length };
}

/**
 * The line from `from`'s hex to `to`'s, `offset` pixels to its right
 * (rule 2), cut at both hexes' edges (rule 3).
 */
function cut(from: Point, to: Point, offset: number): { from: Point; to: Point } {
  const d = unit(from, to);
  const side = { x: -d.y * offset, y: d.x * offset };
  const out = exitPolygon(side, d, OUTLINE);
  const back = exitPolygon(side, { x: -d.x, y: -d.y }, OUTLINE);
  return {
    from: { x: from.x + side.x + d.x * out, y: from.y + side.y + d.y * out },
    to: { x: to.x + side.x - d.x * back, y: to.y + side.y - d.y * back },
  };
}
