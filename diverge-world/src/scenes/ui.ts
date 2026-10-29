// The UI scene: the game's menus, over the world with a camera of its
// own, so they hold still while the map moves. Empty up-front.

import * as Phaser from "phaser";

/** The menus, drawn over the world. */
export class Ui extends Phaser.Scene {
  constructor() {
    super("ui");
  }
}
