// One tile for each kind of entity, drawn once into a texture: a hex
// in the kind's colour, the whole of the entity's hex.

import * as Phaser from "phaser";
import { corners } from "../world/hex";
import type { Kind } from "../world/model";
import { INK, KIND, shade } from "./palette";
import { SIZE } from "./scale";

/** Pixels on a side of a tile's texture: the hex, centred, with room for its outline. */
const CELL = 2 * SIZE + 2;

/** The texture of a kind's tile. */
export const iconKey = (kind: Kind) => `icon:${kind}`;

/** A tile's hex, in its texture's own pixels: what the pointer is over when it is over the entity. */
export const tileShape = (): Phaser.Geom.Polygon => new Phaser.Geom.Polygon(corners(SIZE).map((p) => new Phaser.Math.Vector2(CELL / 2 + p.x, CELL / 2 + p.y)));

const WHITE = 0xffffff;

/**
 * Make the four textures, `CELL` square: the entity's hex filled to its
 * edges in the kind's colour, lit along its upper sides, darkened along
 * its lower ones, and outlined in ink, with a white glyph — a smiley
 * face for an account, a robot's head for an agent, a hammer for a
 * standalone tool, a gear for a dependency tool.
 */
export function makeIcons(scene: Phaser.Scene): void {
  const kinds: Kind[] = ["account", "agent", "standalone_tool", "dependency_tool"];
  const c = CELL / 2;
  const at = (radius: number) => corners(radius).map((p) => new Phaser.Math.Vector2(c + p.x, c + p.y));
  const outer = at(SIZE - 1);
  const inner = at(SIZE - 4);
  for (const kind of kinds) {
    const g = scene.make.graphics({}, false);
    g.fillStyle(KIND[kind], 1);
    g.fillPoints(outer, true);
    // Corners 3, 4 and 5 are the west, north-west and north-east tops: the upper sides catch the light.
    g.lineStyle(3, shade(KIND[kind], 0.35), 1);
    g.strokePoints([inner[3], inner[4], inner[5], inner[0]], false);
    g.lineStyle(3, shade(KIND[kind], -0.3), 1);
    g.strokePoints([inner[0], inner[1], inner[2], inner[3]], false);
    g.lineStyle(2, INK, 1);
    g.strokePoints(outer, true);
    glyph[kind](g, c);
    g.generateTexture(iconKey(kind), CELL, CELL);
    g.destroy();
  }
}

/** Each kind's glyph, about the centre `c`. */
const glyph: Record<Kind, (g: Phaser.GameObjects.Graphics, c: number) => void> = {
  account: (g, c) => {
    g.fillStyle(WHITE, 1);
    g.fillCircle(c, c + 1, 9);
    g.fillStyle(INK, 1);
    g.fillCircle(c - 3.5, c - 1.5, 1.6);
    g.fillCircle(c + 3.5, c - 1.5, 1.6);
    g.lineStyle(1.8, INK, 1);
    g.beginPath();
    g.arc(c, c + 1.5, 4.5, 0.2 * Math.PI, 0.8 * Math.PI);
    g.strokePath();
  },
  agent: (g, c) => {
    g.lineStyle(2, WHITE, 1);
    g.lineBetween(c, c - 9, c, c - 5);
    g.fillStyle(WHITE, 1);
    g.fillCircle(c, c - 10, 2);
    g.fillRoundedRect(c - 8, c - 5, 16, 13, 3);
    g.fillRect(c - 10, c - 1, 2, 5);
    g.fillRect(c + 8, c - 1, 2, 5);
    g.fillStyle(INK, 1);
    g.fillRect(c - 5, c - 2, 3, 3);
    g.fillRect(c + 2, c - 2, 3, 3);
    g.fillRect(c - 4, c + 4, 8, 1.5);
  },
  standalone_tool: (g, c) => {
    g.fillStyle(WHITE, 1);
    const handle = rotated(c, c, Math.PI / 4, [
      [-1.6, -3],
      [1.6, -3],
      [1.6, 11],
      [-1.6, 11],
    ]);
    g.fillPoints(handle, true);
    const head = rotated(c, c, Math.PI / 4, [
      [-8, -9],
      [6, -9],
      [8, -7],
      [8, -3],
      [-8, -3],
    ]);
    g.fillPoints(head, true);
  },
  dependency_tool: (g, c) => {
    const points: Phaser.Math.Vector2[] = [];
    const teeth = 8;
    for (let i = 0; i < teeth * 4; i++) {
      const angle = (i / (teeth * 4)) * Math.PI * 2;
      const r = i % 4 < 2 ? 10 : 7.5;
      points.push(new Phaser.Math.Vector2(c + Math.cos(angle) * r, c + Math.sin(angle) * r));
    }
    g.fillStyle(WHITE, 1);
    g.fillPoints(points, true);
    g.fillStyle(KIND.dependency_tool, 1);
    g.fillCircle(c, c, 3.2);
  },
};

/** The points `[x, y]`, turned by `angle` about the origin, set about `cx`, `cy`. */
function rotated(cx: number, cy: number, angle: number, points: [number, number][]): Phaser.Math.Vector2[] {
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  return points.map(([x, y]) => new Phaser.Math.Vector2(cx + x * cos - y * sin, cy + x * sin + y * cos));
}
