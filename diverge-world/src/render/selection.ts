// The ring round the entity whose sheet is open.

import * as Phaser from "phaser";
import { corners, type Hex } from "../world/hex";
import { pixel, SIZE } from "./scale";

/** The ring's colour, the sheet's copper. */
const COPPER = 0xfbbf24;

/** A ring that is shown at one hex or hidden. */
export type Selection = { show(at: Hex): void; hide(): void };

/**
 * The ring: a copper hex just outside an entity's tile, over the tile
 * and under its name plate. Hidden until shown.
 */
export function makeSelection(scene: Phaser.Scene): Selection {
  const g = scene.add.graphics().setDepth(2.5).setVisible(false);
  const ring = corners(SIZE + 3);
  return {
    show(at) {
      const c = pixel(at);
      g.clear();
      g.lineStyle(3, COPPER, 1);
      g.strokePoints(ring.map((p) => new Phaser.Math.Vector2(c.x + p.x, c.y + p.y)), true);
      g.setVisible(true);
    },
    hide() {
      g.setVisible(false);
    },
  };
}
