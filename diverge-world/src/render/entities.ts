// The entities: a tile filling each one's hex and its name on a plate
// beneath, over the empty land of its ring, as Civilization names a city.

import * as Phaser from "phaser";
import type { WorldMap } from "../world/map";
import type { Entity } from "../world/model";
import { iconKey, tileShape } from "./icons";
import { pixel, SIZE } from "./scale";

/** The most characters a plate shows; a longer name ends in an ellipsis. */
const PLATE = 14;

/**
 * Draw every entity at its hex, and call `hover` with the entity the
 * pointer comes over and with `null` when it leaves.
 */
export function drawEntities(scene: Phaser.Scene, map: WorldMap, hover: (entity: Entity | null) => void): void {
  for (const entity of map.entities) {
    const at = map.at.get(entity.key);
    if (!at) continue;
    const { x: cx, y: cy } = pixel(at);
    const icon = scene.add.image(cx, cy, iconKey(entity.kind)).setDepth(2).setInteractive({ hitArea: tileShape(), hitAreaCallback: Phaser.Geom.Polygon.Contains, useHandCursor: true });
    icon.on("pointerover", () => hover(entity));
    icon.on("pointerout", () => hover(null));
    const name = entity.label.length > PLATE ? `${entity.label.slice(0, PLATE - 1)}…` : entity.label;
    scene.add
      .text(cx, cy + SIZE + 6, name, {
        fontFamily: "Georgia, 'Times New Roman', serif",
        fontSize: "10px",
        color: "#f6eedb",
        backgroundColor: "#1a1510d9",
        padding: { x: 4, y: 1 },
      })
      .setOrigin(0.5)
      .setResolution(3)
      .setDepth(3);
  }
}
