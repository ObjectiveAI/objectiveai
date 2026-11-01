// The nodes: one glowing shape per entity, its name beneath, each
// dragged wherever its owner wants it.

import * as Phaser from "phaser";
import type { Point } from "../world/geometry";
import type { Entity } from "../world/model";
import { css, NEON } from "./palette";
import { HALO, nodeKey, REACH } from "./textures";

/** How far, in pixels, a press may move and still be a click rather than a drag. */
const CLICK = 6;

/** What a dimmed node's shape, name and halo are multiplied by. */
const DIM = 0.25;

/** The most characters a label shows; a longer one ends in an ellipsis. */
const LABEL = 18;

/** One node: its entity, its shape, its name and its halo, which all move together. */
export type Node = { entity: Entity; image: Phaser.GameObjects.Image; label: Phaser.GameObjects.Text; halo: Phaser.GameObjects.Image };

/** The nodes by key, and whether one is being dragged. */
export type Nodes = { byKey: Map<string, Node>; dragging(): boolean };

/** Where a node stands now. */
export const where = (node: Node): Point => ({ x: node.image.x, y: node.image.y });

/** Stand a node at `x`, `y`: its shape, its halo and its name together. */
export function setAt(node: Node, x: number, y: number): void {
  node.image.setPosition(x, y);
  node.halo.setPosition(x, y);
  node.label.setPosition(x, y + REACH[node.entity.kind] + 12);
}

/** Show or hide a node: its shape, its halo and its name, and whether it can be pressed. */
export function setShown(node: Node, shown: boolean): void {
  node.image.setVisible(shown);
  node.halo.setVisible(shown);
  node.label.setVisible(shown);
  if (node.image.input) node.image.input.enabled = shown;
}

/** `color` with each channel multiplied by `by`. */
const scaled = (color: number, by: number) => (Math.round(((color >> 16) & 0xff) * by) << 16) | (Math.round(((color >> 8) & 0xff) * by) << 8) | Math.round((color & 0xff) * by);

/** Dim a node, or bring it back: its shape, its name and its halo darkened together. */
export function setDim(node: Node, dim: boolean): void {
  const tint = dim ? scaled(0xffffff, DIM) : 0xffffff;
  node.image.setTint(tint);
  node.label.setTint(tint);
  node.halo.setTint(dim ? scaled(NEON[node.entity.kind], DIM) : NEON[node.entity.kind]);
}

/**
 * Make the nodes.
 *
 * The rules:
 *
 * 1. Every entity is one node at its first place: its kind's shape,
 *    its halo behind it (unlit until `live` lights it), its name
 *    beneath in its kind's neon.
 * 2. A node is dragged by pressing on it and moving more than `CLICK`
 *    pixels; its halo and name go with it, and every step tells
 *    `moved`. While one is dragged the board does not pan.
 * 3. A press let go having moved less than that is a click, and calls
 *    `select` with an entity that has a log (`subject`) or files
 *    (`files`).
 * 4. The pointer coming onto a node tells `over`, and leaving it tells
 *    `out`.
 * 5. Where nodes stand lives only here, for as long as the page: a
 *    reload stands them at their first places again.
 */
export function makeNodes(scene: Phaser.Scene, entities: Entity[], at: Map<string, Point>, moved: () => void, select: (entity: Entity) => void, over: (node: Node) => void, out: (node: Node) => void): Nodes {
  const byKey = new Map<string, Node>();
  let dragging = false;
  scene.input.dragDistanceThreshold = CLICK;
  for (const entity of entities) {
    const p = at.get(entity.key);
    if (!p) continue;
    // Rule 1.
    const color = NEON[entity.kind];
    const halo = scene.add.image(p.x, p.y, HALO).setTint(color).setAlpha(0).setDepth(4);
    const image = scene.add.image(p.x, p.y, nodeKey(entity.kind)).setDepth(5);
    const name = entity.label.length > LABEL ? `${entity.label.slice(0, LABEL - 1)}…` : entity.label;
    const label = scene.add
      .text(p.x, p.y + REACH[entity.kind] + 12, name, { fontFamily: "Consolas, 'Courier New', monospace", fontSize: "11px", color: css(color) })
      .setOrigin(0.5)
      .setAlpha(0.9)
      .setDepth(6);
    const node = { entity, image, label, halo };
    byKey.set(entity.key, node);
    // Rule 2.
    image.setInteractive({ hitArea: new Phaser.Geom.Circle(image.width / 2, image.height / 2, REACH[entity.kind] + 6), hitAreaCallback: Phaser.Geom.Circle.Contains, draggable: true, useHandCursor: true });
    image.on("dragstart", () => {
      dragging = true;
      image.setDepth(7);
    });
    image.on("drag", (_pointer: Phaser.Input.Pointer, x: number, y: number) => {
      setAt(node, x, y);
      moved();
    });
    image.on("dragend", () => {
      dragging = false;
      image.setDepth(5);
    });
    // Rule 3.
    image.on("pointerup", (pointer: Phaser.Input.Pointer) => {
      if ((entity.subject || entity.files) && Math.hypot(pointer.upX - pointer.downX, pointer.upY - pointer.downY) < CLICK) select(entity);
    });
    // Rule 4.
    image.on("pointerover", () => over(node));
    image.on("pointerout", () => out(node));
  }
  return { byKey, dragging: () => dragging };
}
