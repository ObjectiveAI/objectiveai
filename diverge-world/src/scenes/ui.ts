// The UI scene: the keys. The outline (`outline/outline.ts`) is HTML
// over the canvas; the keyboard does what its presses do.

import * as Phaser from "phaser";
import { TYPES } from "../grid/edges";
import { KIND_ORDER } from "./kinds";
import { flipKind, flipLine } from "../outline/outline";
import { ARRANGE } from "./status";

/**
 * The keys that show and hide each kind, in `KIND_ORDER`, then each type
 * of line, as Phaser names them: the ninth is `9`, the tenth `0`, the
 * eleventh `-`.
 */
const KEYS = ["ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX", "SEVEN", "EIGHT", "NINE", "ZERO", "MINUS"];

/**
 * The keys over the world.
 *
 * The rules:
 *
 * 1. Each of `KEYS` shows or hides its kind or type of line, as the
 *    outline's chips do; the `A` key arranges, as its Arrange does. The
 *    keys are not printed.
 */
export class Ui extends Phaser.Scene {
  constructor() {
    super("ui");
  }

  create() {
    // Rule 1.
    const keyboard = this.input.keyboard;
    if (!keyboard) return;
    KIND_ORDER.forEach(([kind], i) => keyboard.on(`keydown-${KEYS[i]}`, () => flipKind(this.registry, kind)));
    TYPES.forEach((type, i) => keyboard.on(`keydown-${KEYS[KIND_ORDER.length + i]}`, () => flipLine(this.registry, type)));
    keyboard.on("keydown-A", () => this.game.events.emit(ARRANGE));
  }
}
