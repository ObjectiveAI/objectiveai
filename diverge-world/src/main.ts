// The page: one Phaser game filling the window, empty up-front. The
// world scene draws the map beneath; the UI scene draws the menus over
// it. The daemon is reached through `lib/wire.ts` once there is
// something to show.

import * as Phaser from "phaser";
import { Ui } from "./scenes/ui";
import { World } from "./scenes/world";
import "./main.css";

new Phaser.Game({
  type: Phaser.AUTO,
  parent: "world",
  backgroundColor: "#0d0f14",
  scale: { mode: Phaser.Scale.RESIZE, width: "100%", height: "100%" },
  scene: [World, Ui],
});
