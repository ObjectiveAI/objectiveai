// The entities: a tile filling each one's hex and its name on a plate
// beneath, over the empty land of its ring, as Civilization names a city.

import * as Phaser from "phaser";
import type { WorldMap } from "../world/map";
import type { Entity } from "../world/model";
import { iconKey, tileShape } from "./icons";
import { pixel, SIZE } from "./scale";

/** The most characters a plate shows; a longer name ends in an ellipsis. */
const PLATE = 14;

/** How far, in pixels, a press may move and still be a click rather than a drag. */
const CLICK = 6;

/**
 * Draw every entity at its hex, call `hover` with the entity the
 * pointer comes over and with `null` when it leaves, and call `select`
 * with an agent or a listed tool clicked: pressed and let go on its
 * tile, the pointer moved less than `CLICK` pixels between, so a drag
 * that ends on a tile is no click. Accounts, and tools known only from
 * events, are never selected.
 */
export function drawEntities(scene: Phaser.Scene, map: WorldMap, hover: (entity: Entity | null) => void, select: (entity: Entity) => void): void {
  for (const entity of map.entities) {
    const at = map.at.get(entity.key);
    if (!at) continue;
    const { x: cx, y: cy } = pixel(at);
    const icon = scene.add.image(cx, cy, iconKey(entity.kind)).setDepth(2).setInteractive({ hitArea: tileShape(), hitAreaCallback: Phaser.Geom.Polygon.Contains, useHandCursor: true });
    icon.on("pointerover", () => hover(entity));
    icon.on("pointerout", () => hover(null));
    if (entity.agent || entity.tool) {
      icon.on("pointerup", (pointer: Phaser.Input.Pointer) => {
        if (Math.hypot(pointer.upX - pointer.downX, pointer.upY - pointer.downY) < CLICK) select(entity);
      });
    }
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
