// The board's sprites, drawn once into textures with their glow baked
// in, so each node costs one image a frame.

import * as Phaser from "phaser";
import type { Kind } from "../world/model";
import { FIELD, NEON } from "./palette";

/** A node texture's side, in pixels, and the room its glow takes round the shape. */
export const NODE = 64;
const C = NODE / 2;

/** How far from a node's middle a line to it stops: its shape's reach. */
export const REACH: Record<Kind, number> = { account: 17, agent: 20, standalone_tool: 18, connected_standalone_tool: 18, dependency_tool: 13, volume: 15 };

/** Each texture's key. */
export const nodeKey = (kind: Kind) => `grid-${kind}`;
export const HALO = "grid-halo";
export const PULSE = "grid-pulse";

/** A shape traced as points about the middle. */
type Shape = { points: Phaser.Math.Vector2[]; closed: boolean }[];

const v = (x: number, y: number) => new Phaser.Math.Vector2(C + x, C + y);

/** A regular polygon of `n` corners, radius `r`, turned by `turn`. */
const polygon = (n: number, r: number, turn: number) => Array.from({ length: n }, (_, i) => v(r * Math.cos(turn + (i * 2 * Math.PI) / n), r * Math.sin(turn + (i * 2 * Math.PI) / n)));

/** An arc of radius `r` from `a` to `b`, as points. */
const arc = (r: number, a: number, b: number, steps = 24) => Array.from({ length: steps + 1 }, (_, i) => v(r * Math.cos(a + ((b - a) * i) / steps), r * Math.sin(a + ((b - a) * i) / steps)));

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
    { points: arc(13, 0, 2 * Math.PI).map((p) => new Phaser.Math.Vector2(p.x, C + (p.y - C) * 0.35 - 9)), closed: true },
    { points: arc(13, 0, Math.PI).map((p) => new Phaser.Math.Vector2(p.x, C + (p.y - C) * 0.35 + 9)), closed: false },
    { points: [v(-13, -9), v(-13, 9)], closed: false },
    { points: [v(13, -9), v(13, 9)], closed: false },
  ],
};

/** Draw with `paint` into a `w` by `h` texture named `key`, once. */
function make(scene: Phaser.Scene, key: string, w: number, h: number, paint: (g: Phaser.GameObjects.Graphics) => void): void {
  if (scene.textures.exists(key)) return;
  const g = scene.add.graphics();
  paint(g);
  g.generateTexture(key, w, h);
  g.destroy();
}

/** Stroke a shape in `color`: the glow — wide and faint, narrowing — then the bright core. */
function neon(g: Phaser.GameObjects.Graphics, shape: Shape, color: number): void {
  const strokes: [number, number, number][] = [
    [12, color, 0.06],
    [8, color, 0.1],
    [5, color, 0.22],
    [2.5, color, 0.9],
    [1, 0xffffff, 0.85],
  ];
  for (const [width, c, alpha] of strokes) {
    g.lineStyle(width, c, alpha);
    for (const part of shape) g.strokePoints(part.points, part.closed);
  }
}

/**
 * Make every texture:
 *
 * 1. Each kind's node: its outline in its neon, glowing, over a dark
 *    fill so the grid does not show through it.
 * 2. A halo: a soft white ring, tinted by whoever it lights.
 * 3. A pulse: a soft white dot, tinted likewise.
 */
export function makeTextures(scene: Phaser.Scene): void {
  // Rule 1.
  for (const kind of Object.keys(SHAPES) as Kind[]) {
    make(scene, nodeKey(kind), NODE, NODE, (g) => {
      g.fillStyle(FIELD, 0.9);
      g.fillCircle(C, C, REACH[kind] - 1);
      neon(g, SHAPES[kind], NEON[kind]);
    });
  }
  // Rule 2.
  make(scene, HALO, NODE, NODE, (g) => {
    for (let r = 30; r >= 22; r -= 2) {
      g.lineStyle(3, 0xffffff, 0.08 + (30 - r) * 0.02);
      g.strokeCircle(C, C, r);
    }
  });
  // Rule 3.
  make(scene, PULSE, 16, 16, (g) => {
    for (let r = 8; r >= 2; r -= 1.5) {
      g.fillStyle(0xffffff, 0.12 + (8 - r) * 0.1);
      g.fillCircle(8, 8, r);
    }
  });
}
