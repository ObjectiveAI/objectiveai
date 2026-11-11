// The sea scene: the ocean beneath the world, filling the window
// wherever the world's camera looks, so its edge is never seen.

import * as Phaser from "phaser";
import { makeSea, SEA_HEIGHT, SEA_KEY, SEA_WIDTH, SQUEEZE } from "../render/sea";

/**
 * The first scene: it starts the world over itself and draws the sea
 * beneath it, on a camera of its own that never moves.
 *
 * The rules:
 *
 * 1. The sea is one tiling sprite the size of the window, resized with
 *    it.
 * 2. Every frame it reads the world scene's camera: its zoom, and the
 *    world point at the window's top-left — the camera zooms about its
 *    middle, so that point is the scroll, plus half the window, less
 *    half the window at the zoom.
 * 3. The sprite shows that world point at its top-left: the texture
 *    point at the top-left is the world point squeezed (`SQUEEZE`),
 *    within one repeat.
 * 4. One world pixel is `zoom` window pixels, so the texture is scaled
 *    by the zoom, less the squeeze across.
 */
export class Sea extends Phaser.Scene {
  private sea!: Phaser.GameObjects.TileSprite;

  constructor() {
    super("sea");
  }

  create() {
    makeSea(this);
    // Rule 1.
    this.sea = this.add.tileSprite(0, 0, this.scale.width, this.scale.height, SEA_KEY).setOrigin(0, 0);
    this.scale.on("resize", (size: Phaser.Structs.Size) => this.sea.setSize(size.width, size.height));
    this.scene.launch("world");
  }

  update() {
    // Rule 2. Until the world has started, it has no camera to follow, and the sea stays as it was made.
    const world = this.scene.get("world");
    if (!world.sys.isActive()) return;
    const camera = world.cameras.main;
    const zoom = camera.zoom;
    const left = camera.scrollX + camera.width / 2 - camera.width / (2 * zoom);
    const top = camera.scrollY + camera.height / 2 - camera.height / (2 * zoom);
    // Rule 3.
    this.sea.tilePositionX = modulo(left * SQUEEZE, SEA_WIDTH);
    this.sea.tilePositionY = modulo(top, SEA_HEIGHT);
    // Rule 4.
    this.sea.setTileScale(zoom / SQUEEZE, zoom);
  }
}

/** `n` within `[0, period)`. */
const modulo = (n: number, period: number) => ((n % period) + period) % period;
