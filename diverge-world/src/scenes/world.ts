// The world scene: the map, beneath every other scene. It reads the
// daemon once and draws what it found.

import * as Phaser from "phaser";
import { drawEntities } from "../render/entities";
import { makeIcons } from "../render/icons";
import { drawLinks } from "../render/links";
import { makeSelection, type Selection } from "../render/selection";
import { extent } from "../render/scale";
import { drawTerrain } from "../render/terrain";
import { Sheet } from "../sheet/sheet";
import { build, type WorldMap } from "../world/map";
import { type Entity, model } from "../world/model";
import { type Snapshot, snapshot } from "../world/snapshot";
import { controls } from "./camera";
import { type Counts, HOVER, type Hover, STATUS, type Status } from "./status";

/** How long the world waits before it reads the daemon again, after a try that failed: the link's own redial. */
const RETRY = 5000;

/**
 * The map, drawn first; it starts the UI scene over itself.
 *
 * The rules:
 *
 * 1. The daemon is read once (`snapshot`). A read that fails is tried
 *    again `RETRY` milliseconds later, until one does not.
 * 2. What it read is built into a map (`build`) and drawn in order: the
 *    terrain, the links over it, the entities over them.
 * 3. Nothing is read again: the world is the daemon as it stood.
 * 4. Clicking an agent or a listed tool opens its sheet — its log —
 *    and rings its tile; clicking another while one is open swaps the
 *    sheet and the ring to it; clicking the same one again, or the
 *    sheet's ✕, closes it. Nothing else opens or closes it.
 */
export class World extends Phaser.Scene {
  private sheet: Sheet | null = null;
  private selection: Selection | null = null;
  private selected: string | null = null;
  private map: WorldMap | null = null;
  private images = new Map<string, string>();

  constructor() {
    super("world");
  }

  create() {
    this.registry.set(STATUS, { kind: "waiting", reason: null } satisfies Status);
    this.registry.set(HOVER, null satisfies Hover);
    this.scene.launch("ui");
    makeIcons(this);
    void this.take();
  }

  /** Rule 1. */
  private async take() {
    let read: Snapshot;
    try {
      read = await snapshot();
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error);
      this.registry.set(STATUS, { kind: "waiting", reason } satisfies Status);
      this.time.delayedCall(RETRY, () => void this.take());
      return;
    }
    this.draw(read);
  }

  /** Rule 2. */
  private draw(read: Snapshot) {
    const made = model(read);
    const map = build(made);
    this.map = map;
    this.images = made.images;
    drawTerrain(this, map);
    drawLinks(this, map);
    drawEntities(this, map, (entity) => this.registry.set(HOVER, entity satisfies Hover), (entity) => this.select(entity));
    this.selection = makeSelection(this);
    this.sheet = new Sheet(() => this.deselect());
    const size = extent(map.columns, map.rows);
    controls(this, size.width, size.height);
    const counts: Counts = { account: 0, agent: 0, standalone_tool: 0, dependency_tool: 0, links: 0 };
    for (const e of map.entities) counts[e.kind]++;
    for (const p of map.pairs) counts.links += (p.ab ? 1 : 0) + (p.ba ? 1 : 0);
    this.registry.set(STATUS, { kind: "drawn", counts, forbidden: read.forbidden } satisfies Status);
  }

  /** Rule 4. */
  private select(entity: Entity) {
    if (this.selected === entity.key) {
      this.sheet?.close();
      this.deselect();
      return;
    }
    const at = this.map?.at.get(entity.key);
    const subject = entity.subject;
    if (!subject || !at) return;
    this.selected = entity.key;
    this.selection?.show(at);
    this.sheet?.open({ subject, label: entity.label, images: this.images });
  }

  /** Rule 4: the sheet closed, so nothing is selected. */
  private deselect() {
    this.selected = null;
    this.selection?.hide();
  }
}
