// The backdrop scene: the grid beneath the board shown, filling the
// window wherever its camera looks, so its edge is never seen.

import * as Phaser from "phaser";
import { field, gridLine } from "../grid/palette";
import { makeTextures } from "../grid/textures";
import { ACTIVE } from "./status";

/** The world's pixels between fine lines, and how many steps apart the bold ones are. */
const STEP = 40;
const BOLD = 5;

/** Fine lines closer than `FADE_FROM` screen pixels start to fade, and are gone at `FADE_TO`. */
const FADE_FROM = 24;
const FADE_TO = 10;

/**
 * The first scene: it starts the board over itself and draws the grid
 * beneath it, on a camera of its own that never moves.
 *
 * The rules:
 *
 * 1. The field is the camera's own background, the theme's ground.
 * 2. Every frame it reads the camera of the board shown (`ACTIVE`, Home
 *    until a tab is chosen): its zoom, and the world point at the
 *    window's top-left (the scroll, plus half the window, less half the
 *    window at the zoom, as the camera zooms about its middle).
 * 3. The grid is drawn again only when that view or the window has
 *    changed: in the window's own pixels, every line one pixel wide on
 *    a whole pixel, so zooming moves the lines apart and never blurs or
 *    thickens them.
 * 4. Every line is the theme's rule: a bold line every `BOLD` steps,
 *    whole, always shows; the fine lines, at half strength, fade
 *    out as zooming out brings them within `FADE_TO` pixels of each
 *    other, so the grid never crowds into a haze.
 */
export class Backdrop extends Phaser.Scene {
  private grid!: Phaser.GameObjects.Graphics;
  private seen = "";

  constructor() {
    super("backdrop");
  }

  create() {
    makeTextures(this);
    // Rule 1.
    this.cameras.main.setBackgroundColor(field());
    this.grid = this.add.graphics();
    this.scene.launch("board");
  }

  update() {
    // Rule 2. Until the board has started, it has no camera to follow.
    const board = this.scene.get((this.registry.get(ACTIVE) as string | undefined) ?? "board");
    if (!board?.sys.isActive()) return;
    const camera = board.cameras.main;
    const zoom = camera.zoom;
    const left = camera.scrollX + camera.width / 2 - camera.width / (2 * zoom);
    const top = camera.scrollY + camera.height / 2 - camera.height / (2 * zoom);
    const width = this.scale.width;
    const height = this.scale.height;
    // Rule 3.
    const view = `${left} ${top} ${zoom} ${width} ${height}`;
    if (view === this.seen) return;
    this.seen = view;
    const g = this.grid.clear();
    const spacing = STEP * zoom;
    // Rule 4.
    const fine = Phaser.Math.Clamp((spacing - FADE_TO) / (FADE_FROM - FADE_TO), 0, 1);
    const lines = (color: number, alpha: number, every: number) => {
      if (alpha <= 0) return;
      g.lineStyle(1, color, alpha);
      const step = STEP * every;
      for (let x = Math.ceil(left / step) * step; (x - left) * zoom <= width; x += step) {
        if (every === 1 && Math.round(x / STEP) % BOLD === 0) continue;
        const at = Math.round((x - left) * zoom) + 0.5;
        g.lineBetween(at, 0, at, height);
      }
      for (let y = Math.ceil(top / step) * step; (y - top) * zoom <= height; y += step) {
        if (every === 1 && Math.round(y / STEP) % BOLD === 0) continue;
        const at = Math.round((y - top) * zoom) + 0.5;
        g.lineBetween(0, at, width, at);
      }
    };
    lines(gridLine(), fine / 2, 1);
    lines(gridLine(), 1, BOLD);
  }
}
