// The board scene: every entity as a glowing node on the dark grid,
// dragged wherever its owner wants it. It reads the daemon once and
// keeps what it drew alive.

import * as Phaser from "phaser";
import { type Links, makeLinks } from "../grid/links";
import { makeLit } from "../grid/live";
import { makeNodes, type Nodes, where } from "../grid/nodes";
import { place } from "../grid/place";
import { pulses } from "../grid/pulses";
import { REACH } from "../grid/textures";
import { Sheet } from "../sheet/sheet";
import { agentKey, dependencyKey, type Entity, model, toolKey } from "../world/model";
import { type Snapshot, snapshot } from "../world/snapshot";
import { type Watching, watch } from "../world/watch";
import { controls } from "./camera";
import { type Counts, STATUS, type Status } from "./status";

/** How long the board waits before it reads the daemon again, after a try that failed: the link's own redial. */
const RETRY = 5000;

/** The selection ring's colour: white light. */
const RING = 0xffffff;

/**
 * The board, started by the backdrop; it starts the UI scene over
 * itself.
 *
 * The rules:
 *
 * 1. The daemon is read once (`snapshot`). A read that fails is tried
 *    again `RETRY` milliseconds later, until one does not.
 * 2. Every entity is a node at its first place (`place`), the links
 *    under them (`makeLinks`). A node dragged takes its links, its
 *    ring and its pulses with it. One added or removed after waits for
 *    a reload.
 * 3. The nodes are lit from the snapshot and then as the lists kept
 *    open tell (`watch`); pulses run as the events tell (`pulses`).
 * 4. Clicking an agent, a tool or an account opens its sheet — its log
 *    — and rings it; clicking another swaps the sheet and the ring;
 *    clicking the same one again, or the sheet's ✕, closes it.
 */
export class Board extends Phaser.Scene {
  private sheet: Sheet | null = null;
  private ring: Phaser.GameObjects.Arc | null = null;
  private selected: string | null = null;
  private nodes: Nodes | null = null;
  private links: Links | null = null;
  private images = new Map<string, string>();
  private watching: Watching[] = [];

  constructor() {
    super("board");
  }

  create() {
    this.registry.set(STATUS, { kind: "waiting", reason: null } satisfies Status);
    this.scene.launch("ui");
    this.events.once("shutdown", () => {
      for (const w of this.watching) w.stop();
    });
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

  /** Rules 2 and 3. */
  private draw(read: Snapshot) {
    const made = model(read);
    this.images = made.images;
    const placed = place(made.entities);
    this.ring = this.add.circle(0, 0, 30).setStrokeStyle(1.5, RING, 0.9).setVisible(false).setDepth(7);
    const nodes = makeNodes(this, made.entities, placed.at, () => this.moved(), (entity) => this.select(entity));
    this.nodes = nodes;
    this.links = makeLinks(this, nodes, made.pairs);
    // Rule 3.
    const lit = makeLit(this, nodes);
    const deployed = new Map<string, boolean>([
      ...read.agents.map((a) => [agentKey(a), a.deployed] as const),
      ...read.tools.map((t) => [toolKey(t), t.deployed] as const),
      ...read.dependencies.map((d) => [dependencyKey(d), d.deployed] as const),
    ]);
    for (const e of made.entities) if (e.live) lit.set(e.key, e.live, deployed.get(e.key));
    this.watching.push(watch((key, live, up) => lit.set(key, live, up)));
    this.watching.push(pulses(this, nodes, made.pairs, read.events));
    this.sheet = new Sheet(() => this.deselect());
    controls(this, placed.width, placed.height, () => nodes.dragging());
    const counts: Counts = { account: 0, agent: 0, standalone_tool: 0, dependency_tool: 0, volume: 0, links: 0 };
    for (const e of made.entities) counts[e.kind]++;
    for (const p of made.pairs) if (!p.mount) counts.links += (p.ab ? 1 : 0) + (p.ba ? 1 : 0);
    this.registry.set(STATUS, { kind: "drawn", counts, forbidden: read.forbidden } satisfies Status);
  }

  /** Rule 2: a node moved; its links and its ring follow. */
  private moved() {
    this.links?.redraw();
    const node = this.selected === null ? undefined : this.nodes?.byKey.get(this.selected);
    if (node) this.ring?.setPosition(where(node).x, where(node).y);
  }

  /** Rule 4. */
  private select(entity: Entity) {
    if (this.selected === entity.key) {
      this.sheet?.close();
      this.deselect();
      return;
    }
    const node = this.nodes?.byKey.get(entity.key);
    const subject = entity.subject;
    if (!subject || !node || !this.ring) return;
    this.selected = entity.key;
    this.ring.setPosition(node.image.x, node.image.y).setRadius(REACH[entity.kind] + 10).setVisible(true);
    this.sheet?.open({ subject, label: entity.label, images: this.images });
  }

  /** Rule 4: the sheet closed, so nothing is selected. */
  private deselect() {
    this.selected = null;
    this.ring?.setVisible(false);
  }
}
