// Where you are on the board: four gold corners round the node whose
// sheet is open.

import type * as Phaser from "phaser";
import type { Kind } from "../world/model";
import { here } from "./palette";
import { REACH } from "./textures";

/** The corners' width, their arms' length, and how far past the shape's reach they stand, in pixels. */
const WIDTH = 3;
const ARM = 10;
const PAST = 10;

/** The corners: shown round a node of a kind at a place, moved with it, hidden. */
export type Here = { show(kind: Kind, x: number, y: number): void; move(x: number, y: number): void; hide(): void };

/**
 * The corners.
 *
 * The rules:
 *
 * 1. Gold is where you are, and nothing else: four corner marks, square,
 *    `WIDTH` wide with `ARM`-long arms, standing `PAST` beyond the shape's
 *    reach so the trace of a working node runs inside them.
 * 2. They follow their node as it is dragged or glides, and go when its
 *    sheet closes.
 */
export function makeHere(scene: Phaser.Scene): Here {
  const g = scene.add.graphics().setDepth(7).setVisible(false);
  const draw = (kind: Kind) => {
    const h = REACH[kind] + PAST;
    g.clear();
    g.lineStyle(WIDTH, here(), 1);
    for (const [sx, sy] of [[-1, -1], [1, -1], [1, 1], [-1, 1]] as const) {
      g.beginPath();
      g.moveTo(sx * h, sy * (h - ARM));
      g.lineTo(sx * h, sy * h);
      g.lineTo(sx * (h - ARM), sy * h);
      g.strokePath();
    }
  };
  return {
    // Rule 1.
    show(kind, x, y) {
      draw(kind);
      g.setPosition(x, y).setVisible(true);
    },
    // Rule 2.
    move(x, y) {
      g.setPosition(x, y);
    },
    hide() {
      g.setVisible(false);
    },
  };
}
