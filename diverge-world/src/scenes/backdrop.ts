// The backdrop scene: the grid beneath the board, filling the window
// wherever the board's camera looks, so its edge is never seen.

import * as Phaser from "phaser";
import { CELL, CELL_SIZE, makeTextures } from "../grid/textures";

/**
 * The first scene: it starts the board over itself and draws the grid
 * beneath it, on a camera of its own that never moves.
 *
 * The rules:
 *
 * 1. The grid is one tiling sprite the size of the window, resized
 *    with it.
 * 2. Every frame it reads the board's camera: its zoom, and the world
 *    point at the window's top-left (the scroll, plus half the window,
 *    less half the window at the zoom, as the camera zooms about its
 *    middle).
 * 3. The sprite shows that point at its top-left, within one cell, and
 *    is scaled by the zoom.
 */
export class Backdrop extends Phaser.Scene {
  private grid!: Phaser.GameObjects.TileSprite;

  constructor() {
    super("backdrop");
  }

  create() {
    makeTextures(this);
    // Rule 1.
    this.grid = this.add.tileSprite(0, 0, this.scale.width, this.scale.height, CELL).setOrigin(0, 0);
    this.scale.on("resize", (size: Phaser.Structs.Size) => this.grid.setSize(size.width, size.height));
    this.scene.launch("board");
  }

  update() {
    // Rule 2. Until the board has started, it has no camera to follow.
    const board = this.scene.get("board");
    if (!board.sys.isActive()) return;
    const camera = board.cameras.main;
    const zoom = camera.zoom;
    const left = camera.scrollX + camera.width / 2 - camera.width / (2 * zoom);
    const top = camera.scrollY + camera.height / 2 - camera.height / (2 * zoom);
    // Rule 3.
    this.grid.tilePositionX = modulo(left, CELL_SIZE);
    this.grid.tilePositionY = modulo(top, CELL_SIZE);
    this.grid.setTileScale(zoom, zoom);
  }
}

/** `n` within `[0, period)`. */
const modulo = (n: number, period: number) => ((n % period) + period) % period;
