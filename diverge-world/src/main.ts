// The page: one Phaser game filling the window. The backdrop scene
// draws the endless grid beneath; the board scene draws every entity
// over it; the UI scene draws the menus over that. The daemon is
// reached through `lib/wire.ts` once there is something to show. (The
// hex map's scenes, `Sea` and `World`, are kept beside them.)

import * as Phaser from "phaser";
import { FIELD } from "./grid/palette";
import { Backdrop } from "./scenes/backdrop";
import { Board } from "./scenes/board";
import { Ui } from "./scenes/ui";
import "./main.css";

new Phaser.Game({
  type: Phaser.AUTO,
  parent: "world",
  backgroundColor: FIELD,
  scale: { mode: Phaser.Scale.RESIZE, width: "100%", height: "100%" },
  scene: [Backdrop, Board, Ui],
});
