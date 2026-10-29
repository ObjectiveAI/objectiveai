// The entities: a shield on each one's cell and its name on a plate
// beneath, as Civilization names a city.

import * as Phaser from "phaser";
import type { WorldMap } from "../world/map";
import type { Entity } from "../world/model";
import { iconKey } from "./icons";
import { CELL } from "./scale";

/** The most characters a plate shows; a longer name ends in an ellipsis. */
const PLATE = 14;

/**
 * Draw every entity at its cell, and call `hover` with the entity the
 * pointer comes over and with `null` when it leaves.
 */
export function drawEntities(scene: Phaser.Scene, map: WorldMap, hover: (entity: Entity | null) => void): void {
  for (const entity of map.entities) {
    const at = map.at.get(entity.key);
    if (!at) continue;
    const cx = (at.x + 0.5) * CELL;
    const cy = (at.y + 0.5) * CELL;
    const icon = scene.add.image(cx, cy, iconKey(entity.kind)).setDepth(2).setInteractive({ useHandCursor: true });
    icon.on("pointerover", () => hover(entity));
    icon.on("pointerout", () => hover(null));
    const name = entity.label.length > PLATE ? `${entity.label.slice(0, PLATE - 1)}…` : entity.label;
    scene.add
      .text(cx, cy + CELL * 0.95, name, {
        fontFamily: "Georgia, 'Times New Roman', serif",
        fontSize: "11px",
        color: "#f6eedb",
        backgroundColor: "#1a1510d9",
        padding: { x: 4, y: 1 },
      })
      .setOrigin(0.5)
      .setResolution(3)
      .setDepth(3);
  }
}
