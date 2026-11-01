// Looking around the world: fit it to the window, drag to pan, the
// wheel to zoom.

import * as Phaser from "phaser";

const MIN_ZOOM = 0.25;
const MAX_ZOOM = 3;
/** How much one notch of the wheel zooms. */
const STEP = 1.12;

const clampZoom = (zoom: number) => Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, zoom));

/** The part of the world the view's middle may reach, in world pixels. */
export type Extent = { left: number; top: number; right: number; bottom: number };

/**
 * The main camera's controls over a world `width` by `height` pixels;
 * `held`, when given, says something is being dragged; `extent`, when
 * given, says where the world reaches now.
 *
 * The rules:
 *
 * 1. At first the whole world fits the window, with a little sea
 *    around it, and is centred.
 * 2. Dragging with any button pans, unless `held` says something else
 *    is being dragged.
 * 3. The wheel zooms between `MIN_ZOOM` and `MAX_ZOOM`, holding still
 *    the point under the pointer.
 * 4. The middle of the view never leaves the world: `extent` as it is
 *    at each pan and zoom, else the `width` by `height` it began.
 */
export function controls(scene: Phaser.Scene, width: number, height: number, held: () => boolean = () => false, extent: () => Extent = () => ({ left: 0, top: 0, right: width, bottom: height })): void {
  const camera = scene.cameras.main;
  // Rule 4.
  const keepInside = () => {
    const e = extent();
    camera.scrollX = Math.max(e.left - camera.width / 2, Math.min(e.right - camera.width / 2, camera.scrollX));
    camera.scrollY = Math.max(e.top - camera.height / 2, Math.min(e.bottom - camera.height / 2, camera.scrollY));
  };
  // Rule 1.
  camera.setZoom(clampZoom(0.95 * Math.min(camera.width / width, camera.height / height)));
  camera.centerOn(width / 2, height / 2);

  // Rule 2.
  scene.input.on("pointermove", (pointer: Phaser.Input.Pointer) => {
    if (!pointer.isDown || held()) return;
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
