// Looking around the world: fit it to the window, drag to pan, the
// wheel to zoom.

import * as Phaser from "phaser";
import { density, onDensity } from "../lib/density";

const MIN_ZOOM = 0.25;
const MAX_ZOOM = 3;
/** How much one notch of a mouse wheel zooms, and the most one wheel event may. */
const STEP = 1.12;
/** How much a smooth wheel (a trackpad) zooms per pixel scrolled, as a power of e; a pinch, per pixel pinched. */
const SMOOTH = 0.0025;
const PINCH = 0.01;
/** One notch of a mouse wheel in WebKit, in pixels: a wheel whose every move is a whole number of these is a mouse's. */
const NOTCH = 4.000244140625;
/** A move at least this many pixels is a notch of a mouse wheel elsewhere. */
const CLICKY = 50;

/**
 * How much one wheel event zooms, as a factor: a notch of a mouse wheel
 * `STEP`, a trackpad's scroll or pinch as far as it moves, never more
 * than `STEP`; a scroll with no move up or down, nothing.
 */
function wheeled(event: WheelEvent | undefined, dy: number): number {
  const pixels = event?.deltaMode === 1 ? dy * 16 : event?.deltaMode === 2 ? dy * window.innerHeight : dy;
  if (pixels === 0) return 1;
  const notches = Math.abs(pixels) / NOTCH;
  if (Math.abs(pixels) >= CLICKY || Math.abs(notches - Math.round(notches)) < 1e-6) return pixels > 0 ? 1 / STEP : STEP;
  return Math.min(STEP, Math.max(1 / STEP, Math.exp(-pixels * (event?.ctrlKey ? PINCH : SMOOTH))));
}
/** How many times the world's own width and height the view's middle may roam, about the world's middle. */
const REACH = 2;

/** A zoom kept between `MIN_ZOOM` and `MAX_ZOOM` as the page sees them: the game's pixels are the screen's, `density` to each of the page's. */
const clampZoom = (zoom: number) => Math.max(MIN_ZOOM * density(), Math.min(MAX_ZOOM * density(), zoom));

/** The part of the world the view's middle may reach, in world pixels. */
export type Extent = { left: number; top: number; right: number; bottom: number };

/**
 * The main camera's controls over a world `width` by `height` pixels;
 * `held`, when given, says something is being dragged; `extent`, when
 * given, says where the world reaches now; `blocks`, when given, says
 * a point of the window, in the page's pixels, is covered by a menu.
 *
 * The rules:
 *
 * 1. At first the whole world fits the window, with a little sea
 *    around it, and is centred.
 * 2. Dragging with any button pans, unless `held` says something else
 *    is being dragged or the press began where `blocks` says a menu is.
 * 3. The wheel zooms between `MIN_ZOOM` and `MAX_ZOOM`, holding still
 *    the point under the pointer, unless `blocks` says a menu is there:
 *    a mouse wheel `STEP` a notch, a trackpad as far as it scrolls or
 *    pinches (`wheeled`), a sideways scroll not at all.
 * 4. The middle of the view never leaves the world grown `REACH` times
 *    about its own middle: `extent` as it is at each pan and zoom, else
 *    the `width` by `height` it began.
 * 5. The window moving to a screen of another density keeps the view:
 *    the same middle, and the zoom scaled with the density, so the
 *    world looks the size it did.
 */
export function controls(scene: Phaser.Scene, width: number, height: number, held: () => boolean = () => false, extent: () => Extent = () => ({ left: 0, top: 0, right: width, bottom: height }), blocks: (x: number, y: number) => boolean = () => false): void {
  const camera = scene.cameras.main;
  // Rule 4.
  const keepInside = () => {
    const world = extent();
    const grow = (REACH - 1) / 2;
    const dx = (world.right - world.left) * grow;
    const dy = (world.bottom - world.top) * grow;
    const e = { left: world.left - dx, top: world.top - dy, right: world.right + dx, bottom: world.bottom + dy };
    camera.scrollX = Math.max(e.left - camera.width / 2, Math.min(e.right - camera.width / 2, camera.scrollX));
    camera.scrollY = Math.max(e.top - camera.height / 2, Math.min(e.bottom - camera.height / 2, camera.scrollY));
  };
  // Rule 1.
  camera.setZoom(clampZoom(0.95 * Math.min(camera.width / width, camera.height / height)));
  camera.centerOn(width / 2, height / 2);

  // Rule 2.
  scene.input.on("pointermove", (pointer: Phaser.Input.Pointer) => {
    if (!pointer.isDown || held() || blocks(pointer.downX / density(), pointer.downY / density())) return;
    camera.scrollX -= (pointer.x - pointer.prevPosition.x) / camera.zoom;
    camera.scrollY -= (pointer.y - pointer.prevPosition.y) / camera.zoom;
    keepInside();
  });

  // Rule 3.
  scene.input.on("wheel", (pointer: Phaser.Input.Pointer, _over: unknown, _dx: number, dy: number) => {
    if (blocks(pointer.x / density(), pointer.y / density())) return;
    const by = wheeled(pointer.event as WheelEvent | undefined, dy);
    if (by === 1) return;
    const before = camera.getWorldPoint(pointer.x, pointer.y);
    camera.setZoom(clampZoom(camera.zoom * by));
    camera.preRender();
    const after = camera.getWorldPoint(pointer.x, pointer.y);
    camera.scrollX += before.x - after.x;
    camera.scrollY += before.y - after.y;
    keepInside();
  });

  // Rule 5.
  const off = onDensity((next, was) => {
    const middle = { x: camera.midPoint.x, y: camera.midPoint.y };
    camera.setZoom(camera.zoom * (next / was));
    camera.centerOn(middle.x, middle.y);
  });
  scene.events.once(Phaser.Scenes.Events.DESTROY, off);
}
