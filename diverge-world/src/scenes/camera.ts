// Looking around the world: fit it to the window, drag to pan, the
// wheel to zoom.

import * as Phaser from "phaser";

const MIN_ZOOM = 0.25;
const MAX_ZOOM = 3;
/** How much one notch of the wheel zooms. */
const STEP = 1.12;

const clampZoom = (zoom: number) => Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, zoom));

/**
 * The main camera's controls over a world `width` by `height` pixels.
 *
 * The rules:
 *
 * 1. At first the whole world fits the window, with a little sea
 *    around it, and is centred.
 * 2. Dragging with any button pans.
 * 3. The wheel zooms between `MIN_ZOOM` and `MAX_ZOOM`, holding still
 *    the point under the pointer.
 * 4. The middle of the view never leaves the world.
 */
export function controls(scene: Phaser.Scene, width: number, height: number): void {
  const camera = scene.cameras.main;
  // Rule 4.
  const keepInside = () => {
    camera.scrollX = Math.max(-camera.width / 2, Math.min(width - camera.width / 2, camera.scrollX));
    camera.scrollY = Math.max(-camera.height / 2, Math.min(height - camera.height / 2, camera.scrollY));
  };
  // Rule 1.
  camera.setZoom(clampZoom(0.95 * Math.min(camera.width / width, camera.height / height)));
  camera.centerOn(width / 2, height / 2);

  // Rule 2.
  scene.input.on("pointermove", (pointer: Phaser.Input.Pointer) => {
    if (!pointer.isDown) return;
    camera.scrollX -= (pointer.x - pointer.prevPosition.x) / camera.zoom;
    camera.scrollY -= (pointer.y - pointer.prevPosition.y) / camera.zoom;
    keepInside();
  });

  // Rule 3.
  scene.input.on("wheel", (pointer: Phaser.Input.Pointer, _over: unknown, _dx: number, dy: number) => {
    const before = camera.getWorldPoint(pointer.x, pointer.y);
    camera.setZoom(clampZoom(dy > 0 ? camera.zoom / STEP : camera.zoom * STEP));
    camera.preRender();
    const after = camera.getWorldPoint(pointer.x, pointer.y);
    camera.scrollX += before.x - after.x;
    camera.scrollY += before.y - after.y;
    keepInside();
  });
}
