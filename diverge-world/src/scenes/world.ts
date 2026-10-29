// The world scene: the map, beneath every other scene. Empty up-front.

import * as Phaser from "phaser";

/** The map, drawn first; it starts the UI scene over itself. */
export class World extends Phaser.Scene {
  constructor() {
    super("world");
  }

  create() {
    this.scene.launch("ui");
  }
}
