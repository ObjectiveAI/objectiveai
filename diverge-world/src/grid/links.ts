// The links: light lines between nodes, redrawn when a node moves.

import * as Phaser from "phaser";
import type { Point } from "../world/geometry";
import type { Pair } from "../world/model";
import { type Node, type Nodes, where } from "./nodes";
import { NEON } from "./palette";
import { REACH } from "./textures";

/** How far apart the two lines of a two-way pair run, from the middle, in pixels. */
const APART = 4;

/** A mount's dash and gap, in pixels. */
const DASH = 7;
const GAP = 5;

/** The chevron's length and half-width, in pixels. */
const HEAD = 9;
const WING = 5;

/** The passes a line is drawn in: the glow, wide and faint, then the core. */
const PASSES: [number, number][] = [
  [8, 0.07],
  [4, 0.18],
  [1.6, 0.95],
];

/** The links, drawn again on `redraw`. */
export type Links = { redraw(): void };

/**
 * The line from `from` to `to` as it is drawn: stopped short of each
 * node's shape, and moved `offset` pixels to its left. `null` when the
 * two are too near to draw.
 */
export function segment(from: Node, to: Node, offset: number): { a: Point; b: Point } | null {
  const p = where(from);
  const q = where(to);
  const length = Math.hypot(q.x - p.x, q.y - p.y);
  const sa = REACH[from.entity.kind] + 5;
  const sb = REACH[to.entity.kind] + 5;
  if (length <= sa + sb) return null;
  const d = { x: (q.x - p.x) / length, y: (q.y - p.y) / length };
  const n = { x: d.y * offset, y: -d.x * offset };
  return { a: { x: p.x + d.x * sa + n.x, y: p.y + d.y * sa + n.y }, b: { x: q.x - d.x * sb + n.x, y: q.y - d.y * sb + n.y } };
}

/** How far to the left a pair's line from `from` runs: apart when both ways ran, else on the middle. */
export const offsetOf = (pair: Pair) => (pair.ab && pair.ba ? APART : 0);

/**
 * The links over the field and under the nodes.
 *
 * The rules:
 *
 * 1. Every way a pair ran is one line from its sender to its receiver,
 *    in the sender's neon, glowing (`PASSES`), with a chevron at the
 *    receiver's end. A pair that ran both ways is two lines, each
 *    `APART` to its own left.
 * 2. A mount is a dashed line in the volume's neon, with no chevron.
 * 3. Every line stops short of the shapes it joins.
 * 4. They are drawn again only when a node moves.
 */
export function makeLinks(scene: Phaser.Scene, nodes: Nodes, pairs: Pair[]): Links {
  const g = scene.add.graphics().setDepth(3);
  const redraw = () => {
    g.clear();
    for (const pair of pairs) {
      const a = nodes.byKey.get(pair.a);
      const b = nodes.byKey.get(pair.b);
      if (!a || !b) continue;
      // Rule 2.
      if (pair.mount) {
        const s = segment(a, b, 0);
        if (!s) continue;
        g.lineStyle(1.6, NEON.volume, 0.7);
        dashed(g, s.a, s.b);
        continue;
      }
      // Rule 1.
      const ways: [Node, Node][] = [];
      if (pair.ab) ways.push([a, b]);
      if (pair.ba) ways.push([b, a]);
      for (const [from, to] of ways) {
        const s = segment(from, to, offsetOf(pair));
        if (!s) continue;
        const color = NEON[from.entity.kind];
        const length = Math.hypot(s.b.x - s.a.x, s.b.y - s.a.y);
        const d = { x: (s.b.x - s.a.x) / length, y: (s.b.y - s.a.y) / length };
        const base = { x: s.b.x - d.x * HEAD, y: s.b.y - d.y * HEAD };
        for (const [width, alpha] of PASSES) {
          g.lineStyle(width, color, alpha);
          g.lineBetween(s.a.x, s.a.y, s.b.x, s.b.y);
          g.lineBetween(s.b.x, s.b.y, base.x + d.y * WING, base.y - d.x * WING);
          g.lineBetween(s.b.x, s.b.y, base.x - d.y * WING, base.y + d.x * WING);
        }
      }
    }
  };
  redraw();
  return { redraw };
}

/** A dashed line from `a` to `b`, in the graphics' line style. */
function dashed(g: Phaser.GameObjects.Graphics, a: Point, b: Point): void {
  const length = Math.hypot(b.x - a.x, b.y - a.y);
  const d = { x: (b.x - a.x) / length, y: (b.y - a.y) / length };
  for (let at = 0; at < length; at += DASH + GAP) {
    const end = Math.min(at + DASH, length);
    g.lineBetween(a.x + d.x * at, a.y + d.y * at, a.x + d.x * end, a.y + d.y * end);
  }
}
