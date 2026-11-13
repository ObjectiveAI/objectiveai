// Looking around the world: fit it to the window; drag, or swipe two
// fingers on a trackpad, to pan; a mouse wheel, or a pinch, to zoom.

import * as Phaser from "phaser";
import { density, onDensity } from "../lib/density";

const MIN_ZOOM = 0.25;
const MAX_ZOOM = 3;
/** How much one notch of a mouse wheel zooms, and the most one pinch event may. */
const STEP = 1.12;
/** How much a pinch reported as a wheel with ctrl held zooms per pixel, as a power of e. */
const PINCH = 0.01;
/** One notch of a mouse wheel in WebKit, in pixels: a wheel whose every move is a whole number of these is a mouse's. */
const NOTCH = 4.000244140625;
/** A move up or down of at least this many whole pixels, and none sideways, is a notch of a mouse wheel elsewhere. */
const CLICKY = 50;

/** Whether a wheel event is a mouse wheel's notch rather than a trackpad's two-finger scroll. */
function notched(event: WheelEvent | undefined, dx: number, dy: number): boolean {
  if (event !== undefined && event.deltaMode !== 0) return true;
  if (dx !== 0 || dy === 0) return false;
  const notches = Math.abs(dy) / NOTCH;
  return Math.abs(notches - Math.round(notches)) < 1e-6 || (Math.abs(dy) >= CLICKY && Number.isInteger(dy));
}

/** A pinch as WebKit tells it: how far it has scaled since it began, about the point between the fingers. */
type Gesture = UIEvent & { scale: number; clientX?: number; clientY?: number };
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
 * 3. Unless `blocks` says a menu is there: a mouse wheel zooms `STEP` a
 *    notch (`notched`), holding still the point under the pointer; a
 *    trackpad's two-finger scroll pans as far as the fingers move; a
 *    pinch zooms as far as it pinches, about the point between the
 *    fingers — WebKit's gesture events, or a wheel with ctrl held where
 *    a pinch comes as one. Zoom stays between `MIN_ZOOM` and `MAX_ZOOM`.
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

  // Rule 3: zoom to `to`, holding still the game's point `x`, `y`.
  const zoomAt = (to: number, x: number, y: number) => {
    const before = camera.getWorldPoint(x, y);
    camera.setZoom(clampZoom(to));
    camera.preRender();
    const after = camera.getWorldPoint(x, y);
    camera.scrollX += before.x - after.x;
    camera.scrollY += before.y - after.y;
    keepInside();
  };
  let pinchFrom: number | null = null;
  scene.input.on("wheel", (pointer: Phaser.Input.Pointer, _over: unknown, dx: number, dy: number) => {
    if (blocks(pointer.x / density(), pointer.y / density())) return;
    const event = pointer.event as WheelEvent | undefined;
    if (event?.ctrlKey) {
      if (pinchFrom === null) zoomAt(camera.zoom * Math.min(STEP, Math.max(1 / STEP, Math.exp(-dy * PINCH))), pointer.x, pointer.y);
      return;
    }
    if (notched(event, dx, dy)) {
      if (dy !== 0) zoomAt(dy > 0 ? camera.zoom / STEP : camera.zoom * STEP, pointer.x, pointer.y);
      return;
    }
    // A wheel's move is in the page's pixels; the game's are the screen's.
    camera.scrollX += (dx * density()) / camera.zoom;
    camera.scrollY += (dy * density()) / camera.zoom;
    keepInside();
  });
  const canvas = scene.game.canvas;
  const between = (gesture: Gesture) => {
    if (gesture.clientX === undefined || gesture.clientY === undefined) return { x: scene.input.activePointer.x, y: scene.input.activePointer.y };
    const box = canvas.getBoundingClientRect();
    return { x: (gesture.clientX - box.left) * density(), y: (gesture.clientY - box.top) * density() };
  };
  const pinchStart = (event: Event) => {
    event.preventDefault();
    const at = between(event as Gesture);
    pinchFrom = scene.sys.isActive() && !blocks(at.x / density(), at.y / density()) ? camera.zoom : null;
  };
  const pinchChange = (event: Event) => {
    event.preventDefault();
    if (pinchFrom === null || !scene.sys.isActive()) return;
    const at = between(event as Gesture);
    zoomAt(pinchFrom * (event as Gesture).scale, at.x, at.y);
  };
  const pinchEnd = (event: Event) => {
    event.preventDefault();
    pinchFrom = null;
  };
  canvas.addEventListener("gesturestart", pinchStart);
  canvas.addEventListener("gesturechange", pinchChange);
  canvas.addEventListener("gestureend", pinchEnd);
  scene.events.once(Phaser.Scenes.Events.DESTROY, () => {
    canvas.removeEventListener("gesturestart", pinchStart);
    canvas.removeEventListener("gesturechange", pinchChange);
    canvas.removeEventListener("gestureend", pinchEnd);
  });

  // Rule 5.
  const off = onDensity((next, was) => {
    const middle = { x: camera.midPoint.x, y: camera.midPoint.y };
    camera.setZoom(camera.zoom * (next / was));
    camera.centerOn(middle.x, middle.y);
  });
  scene.events.once(Phaser.Scenes.Events.DESTROY, off);
}
