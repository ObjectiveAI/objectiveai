// The page: one Phaser game filling the window. The backdrop scene
// draws the endless grid beneath; the board scene draws every entity
// over it; the UI scene takes the keys. The outline floats over the
// board in HTML, and the tab strip along the top adds a board per daemon
// opened, and switches among them. The daemon is reached through
// `lib/wire.ts` once there is something to show. (The hex map's scenes,
// `Sea` and `World`, are kept beside them.)

import "../../diverge-theme/theme.css";
import "./main.css";
import * as Phaser from "phaser";
import { int } from "./lib/theme";
import { outline } from "./outline/outline";
import { Backdrop } from "./scenes/backdrop";
import { Board } from "./scenes/board";
import { Ui } from "./scenes/ui";
import { tabs } from "./tabs/tabs";

/** The keys that move among things on the page: while one is used, hover marks nothing. */
const MOVING = new Set(["Tab", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Home", "End", "PageUp", "PageDown"]);

/**
 * Which hand is in use, on the root as `data-input`: `keys` from a key
 * that moves among things until the mouse moves, `mouse` otherwise. The
 * page's styles mark hover only for the mouse, so the keys' focus and
 * the mouse's hover never mark two things at once.
 */
function hands(): void {
  const root = document.documentElement;
  root.dataset.input = "mouse";
  addEventListener("keydown", (event) => MOVING.has(event.key) && (root.dataset.input = "keys"), true);
  addEventListener("pointermove", () => root.dataset.input !== "mouse" && (root.dataset.input = "mouse"), true);
}

hands();
const game = new Phaser.Game({
  type: Phaser.AUTO,
  parent: "world",
  backgroundColor: int("bg"),
  scale: { mode: Phaser.Scale.RESIZE, width: "100%", height: "100%" },
  scene: [Backdrop, Board, Ui],
});
tabs(game);
outline(game);
