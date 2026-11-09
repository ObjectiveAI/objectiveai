// The tab strip: one tab per daemon the window shows, along its top, in
// HTML over the canvas — Home, our own daemon, first, then every daemon
// opened from a daemon's sheet, each its own board by the route to it.

import type * as Phaser from "phaser";
import type { Route } from "../lib/wire";
import { Board } from "../scenes/board";
import { ACTIVE, OPEN, type Opening } from "../scenes/status";
import "./tabs.css";

/** The strip's height, in pixels: the outliner stands this much lower. */
export const TABS = 30;

/** Home's board: the one the backdrop starts, its route `[]`. */
const HOME = "board";

/** A board's scene key, by its route: `board:<id>/<id>…`. */
const keyOf = (route: Route) => `${HOME}:${route.join("/")}`;

/** One opened tab: its board's scene key, and the route and names it was opened with. */
type Tab = { key: string } & Opening;

/**
 * Start the tab strip over `game`.
 *
 * The rules:
 *
 * 1. It stands along the top of the window, `TABS` tall, the full
 *    width, over the canvas and under the sheet. Nothing pressed on it
 *    reaches the board beneath: a node and the outliner take a press
 *    only when it went down and came up on the canvas.
 * 2. Home is the first tab and has no ✕; every other is named by the
 *    names along its route (`A › B`), its route's IDs its tooltip, and
 *    has a ✕. The tab shown (`ACTIVE`, Home at the start) is lit.
 * 3. A daemon's sheet asks for a tab (`OPEN`): a route that has one
 *    already is switched to; else its board is added — a `Board` keyed
 *    by its route (`keyOf`), given its `Opening` — and shown, the board
 *    shown before put to sleep as a switch does, and the UI scene
 *    brought back over every board.
 * 4. Switching puts the board shown to sleep, which closes its sheet;
 *    names the target shown (`ACTIVE`); and wakes it, so it tells the
 *    outliner its own again. A board asleep keeps its lists open and
 *    its world current, so switching back loses nothing.
 * 5. ✕ closes a tab: if it is shown, the tab to its left is shown
 *    first; then its board is stopped — its lists closed, its sheet out
 *    of the page — and removed.
 */
export function tabs(game: Phaser.Game): void {
  const opened: Tab[] = [];
  game.registry.set(ACTIVE, HOME);
  // Rule 1.
  const strip = document.createElement("nav");
  strip.className = "window-tabs";
  strip.style.height = `${TABS}px`;
  document.body.append(strip);
  const shown = () => (game.registry.get(ACTIVE) as string | undefined) ?? HOME;
  // Rule 2.
  const draw = () => {
    const tab = (key: string, words: string, title: string, close: (() => void) | null) => {
      const made = document.createElement("div");
      made.className = "tab";
      made.classList.toggle("on", key === shown());
      made.title = title;
      const name = document.createElement("span");
      name.textContent = words;
      made.append(name);
      made.addEventListener("click", () => show(key));
      if (close) {
        const exit = document.createElement("button");
        exit.type = "button";
        exit.className = "close";
        exit.textContent = "✕";
        exit.title = "Close";
        exit.addEventListener("click", (event) => {
          event.stopPropagation();
          close();
        });
        made.append(exit);
      }
      return made;
    };
    strip.replaceChildren(
      tab(HOME, "Home", "Our own daemon", null),
      ...opened.map((t) => tab(t.key, t.labels.join(" › "), t.route.join(" › "), () => close(t))),
    );
  };
  // Rule 4.
  const show = (key: string) => {
    const from = shown();
    if (from === key) return;
    game.scene.sleep(from);
    game.registry.set(ACTIVE, key);
    game.scene.wake(key);
    draw();
  };
  // Rule 3.
  const open = (opening: Opening) => {
    const key = keyOf(opening.route);
    if (opened.some((t) => t.key === key)) return show(key);
    opened.push({ key, ...opening });
    game.scene.sleep(shown());
    game.registry.set(ACTIVE, key);
    game.scene.add(key, new Board(key), true, opening);
    game.scene.bringToTop("ui");
    draw();
  };
  // Rule 5.
  const close = (tab: Tab) => {
    const at = opened.indexOf(tab);
    if (at === -1) return;
    if (shown() === tab.key) show(at === 0 ? HOME : opened[at - 1].key);
    opened.splice(at, 1);
    game.scene.stop(tab.key);
    game.scene.remove(tab.key);
    draw();
  };
  game.events.on(OPEN, open);
  draw();
}
