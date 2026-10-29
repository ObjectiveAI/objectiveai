// The page: one Phaser game filling the window. The sea scene draws
// the endless ocean beneath; the world scene draws the map over it; the
// UI scene draws the menus over that. The daemon is reached through `lib/wire.ts` once there is
// something to show.

import * as Phaser from "phaser";
import { SEA } from "./render/palette";
import { Sea } from "./scenes/sea";
import { Ui } from "./scenes/ui";
import { World } from "./scenes/world";
import "./main.css";

new Phaser.Game({
  type: Phaser.AUTO,
  parent: "world",
  backgroundColor: SEA,
  scale: { mode: Phaser.Scale.RESIZE, width: "100%", height: "100%" },
  scene: [Sea, World, Ui],
});
