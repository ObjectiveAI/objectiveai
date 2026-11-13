// The board's sprites, drawn once into textures, so each node costs one
// image a frame: every kind's shape, flat, bright and faint.

import * as Phaser from "phaser";
import type { Kind } from "../world/model";
import { faint, field, tone } from "./palette";

/** A node's side on the board, in pixels. */
export const NODE = 64;
const C = NODE / 2;

/** How many of a texture's pixels each of the board's holds: the shapes are drawn this much finer and shown this much smaller, so they stay sharp zoomed in. */
export const DETAIL = 4;

/** The width of a shape's stroke, in pixels: flat, no glow. */
const STROKE = 2;

/** How far from a node's middle a line to it stops: its shape's reach. */
export const REACH: Record<Kind, number> = { account: 17, agent: 20, standalone_tool: 18, connected_standalone_tool: 18, dependency_tool: 13, volume: 15, provider: 19, daemon: 16 };

/** Each texture's key: a kind's shape bright, or faint. */
export const nodeKey = (kind: Kind, bright = true) => `grid-${kind}${bright ? "" : "-faint"}`;

/** A shape traced as points about the middle. */
type Shape = { points: Phaser.Math.Vector2[]; closed: boolean }[];

const v = (x: number, y: number) => new Phaser.Math.Vector2(C + x, C + y);

/** A regular polygon of `n` corners, radius `r`, turned by `turn`. */
const polygon = (n: number, r: number, turn: number) => Array.from({ length: n }, (_, i) => v(r * Math.cos(turn + (i * 2 * Math.PI) / n), r * Math.sin(turn + (i * 2 * Math.PI) / n)));

/** An arc of radius `r` from `a` to `b`, as points. */
const arc = (r: number, a: number, b: number, steps = 24) => Array.from({ length: steps + 1 }, (_, i) => v(r * Math.cos(a + ((b - a) * i) / steps), r * Math.sin(a + ((b - a) * i) / steps)));

/** A squashed arc: the cylinder's top and foot, flattened to `squash` and lifted by `dy`. */
const flat = (r: number, a: number, b: number, squash: number, dy: number) => arc(r, a, b).map((p) => new Phaser.Math.Vector2(p.x, C + (p.y - C) * squash + dy));

/** Each kind's outline. */
const SHAPES: Record<Kind, Shape> = {
  agent: [
    { points: polygon(6, 20, -Math.PI / 2), closed: true },
    { points: polygon(6, 8, -Math.PI / 2), closed: true },
  ],
  standalone_tool: [
    { points: polygon(4, 19, -Math.PI / 2), closed: true },
    { points: [v(-7, 0), v(7, 0)], closed: false },
  ],
  // The standalone tool's diamond, held by a link: two rings through each other.
  connected_standalone_tool: [
    { points: polygon(4, 19, -Math.PI / 2), closed: true },
    { points: arc(4.5, 0, 2 * Math.PI, 16).map((p) => new Phaser.Math.Vector2(p.x - 3.5, p.y)), closed: true },
    { points: arc(4.5, 0, 2 * Math.PI, 16).map((p) => new Phaser.Math.Vector2(p.x + 3.5, p.y)), closed: true },
  ],
  dependency_tool: [{ points: polygon(4, 13, Math.PI / 4), closed: true }],
  account: [
    { points: arc(17, -Math.PI * 0.35, Math.PI * 1.35), closed: false },
    { points: arc(7, 0, 2 * Math.PI), closed: true },
  ],
  volume: [
    { points: flat(13, 0, 2 * Math.PI, 0.35, -9), closed: true },
    { points: flat(13, 0, Math.PI, 0.35, 9), closed: false },
    { points: [v(-13, -9), v(-13, 9)], closed: false },
    { points: [v(13, -9), v(13, 9)], closed: false },
  ],
  // A server rack: a tall block of three units, each with its light.
  provider: [
    { points: [v(-11, -16), v(11, -16), v(11, 16), v(-11, 16)], closed: true },
    { points: [v(-11, -5.5), v(11, -5.5)], closed: false },
    { points: [v(-11, 5.5), v(11, 5.5)], closed: false },
    { points: [v(-6, -11), v(-2, -11)], closed: false },
    { points: [v(-6, 0), v(-2, 0)], closed: false },
    { points: [v(-6, 11), v(-2, 11)], closed: false },
  ],
  // A globe: its ring, its meridian, its equator and two parallels — another daemon, far off.
  daemon: [
    { points: arc(16, 0, 2 * Math.PI), closed: true },
    { points: arc(16, 0, 2 * Math.PI).map((p) => new Phaser.Math.Vector2(C + (p.x - C) * 0.42, p.y)), closed: true },
    { points: [v(-16, 0), v(16, 0)], closed: false },
    { points: [v(-13.9, -8), v(13.9, -8)], closed: false },
    { points: [v(-13.9, 8), v(13.9, 8)], closed: false },
  ],
};

/** A point about the node's middle, as `{x, y}` from it. */
export type Offset = { x: number; y: number };

/**
 * Each kind's outline as one path about its middle, for the trace that
 * runs round a working node (`live`): the shape's outer line, start to
 * end, closed back to its start where the shape is closed. A volume's is
 * its silhouette: down one side, round the front of its foot, up the
 * other, and across the back of its top.
 */
export const OUTLINE: Record<Kind, Offset[]> = (() => {
  const off = (points: Phaser.Math.Vector2[]) => points.map((p) => ({ x: p.x - C, y: p.y - C }));
  const closed = (points: Phaser.Math.Vector2[]) => [...off(points), off(points)[0]];
  const made = {} as Record<Kind, Offset[]>;
  for (const kind of Object.keys(SHAPES) as Kind[]) {
    const outer = SHAPES[kind][0];
    made[kind] = outer.closed ? closed(outer.points) : off(outer.points);
  }
  made.volume = off([v(-13, -9), ...flat(13, Math.PI, 0, 0.35, 9), v(13, -9), ...flat(13, 0, -Math.PI, 0.35, -9)]);
  return made;
})();

/** Draw with `paint` into a `w` by `h` texture named `key`, once. */
function make(scene: Phaser.Scene, key: string, w: number, h: number, paint: (g: Phaser.GameObjects.Graphics) => void): void {
  if (scene.textures.exists(key)) return;
  const g = scene.add.graphics();
  paint(g);
  g.generateTexture(key, w, h);
  g.destroy();
}

/**
 * Make every texture: each kind's node, twice.
 *
 * The rules:
 *
 * 1. Its outline flat, `STROKE` wide, over a disc of the ground so the
 *    grid does not show through it. No glow: light means something on
 *    this board (a working node's trace), so nothing glows at rest.
 * 2. Bright, in its family's colour; faint, in the strong edge — a
 *    still shape that reads at 3:1, never a fade.
 * 3. Each is drawn `DETAIL` times finer than the board shows it, and
 *    the game keeps smaller copies of it (mipmaps, `main.ts`), so a
 *    node is sharp zoomed in and steady zoomed out.
 */
export function makeTextures(scene: Phaser.Scene): void {
  for (const kind of Object.keys(SHAPES) as Kind[]) {
    // Rule 2.
    for (const bright of [true, false]) {
      make(scene, nodeKey(kind, bright), NODE * DETAIL, NODE * DETAIL, (g) => {
        // Rule 1.
        g.fillStyle(field(), 1);
        g.fillCircle(C * DETAIL, C * DETAIL, (REACH[kind] - 1) * DETAIL);
        g.lineStyle(STROKE * DETAIL, bright ? tone(kind) : faint(), 1);
        for (const part of SHAPES[kind]) g.strokePoints(part.points.map((p) => new Phaser.Math.Vector2(p.x * DETAIL, p.y * DETAIL)), part.closed);
      });
    }
  }
}
