// The board scene: every entity as a glowing node on the dark grid,
// dragged wherever its owner wants it. It reads the daemon once and
// keeps what it drew alive.

import * as Phaser from "phaser";
import { arrange } from "../grid/arrange";
import { type Edge, edges } from "../grid/edges";
import { type Links, makeLinks } from "../grid/links";
import { makeLit } from "../grid/live";
import { makeNodes, type Node, type Nodes, setAt, setDim, setShown, where } from "../grid/nodes";
import { place } from "../grid/place";
import { pulses } from "../grid/pulses";
import { REACH } from "../grid/textures";
import { Sheet } from "../sheet/sheet";
import { accountKey, agentKey, dependencyKey, type Entity, model, toolKey, volumeKey } from "../world/model";
import { mountsOf } from "../world/mounts";
import { type Snapshot, snapshot } from "../world/snapshot";
import { type Watching, watch } from "../world/watch";
import { controls } from "./camera";
import { ARRANGE, type Counts, KINDS, type Kinds, LINES, type Lines, STATUS, type Status } from "./status";

/** How long the board waits before it reads the daemon again, after a try that failed: the link's own redial. */
const RETRY = 5000;

/** The selection ring's colour: white light. */
const RING = 0xffffff;

/** How long nodes take to glide to their arranged places, in milliseconds. */
const GLIDE = 700;

/**
 * The board, started by the backdrop; it starts the UI scene over
 * itself.
 *
 * The rules:
 *
 * 1. The daemon is read once (`snapshot`). A read that fails is tried
 *    again `RETRY` milliseconds later, until one does not.
 * 2. Every entity is a node at its first place (`place`), every edge
 *    (`edges`) — call, attachment, creator — a line under them, side by
 *    side in lanes (`makeLinks`). A node dragged takes its lines, its
 *    ring and its pulses with it. One added or removed after waits for
 *    a reload. The legend shows and hides each type of line (`LINES`)
 *    and each kind of node (`KINDS`); a hidden node takes its lines
 *    and pulses with it, and closes its sheet if it was open.
 * 3. The nodes are lit from the snapshot and then as the lists kept
 *    open tell (`watch`); pulses run as the events tell (`pulses`).
 * 4. Clicking an agent, a tool, an account or a volume opens its sheet —
 *    its log, its files, or both — and rings it; clicking another swaps the sheet and the ring;
 *    clicking the same one again, or the sheet's ✕, closes it.
 * 5. Arrange (`ARRANGE`) gives every node shown a new place by the
 *    lines shown (`arrange`), and the nodes glide there over `GLIDE`, their
 *    lines and ring following. It does nothing while a node is
 *    dragged, or while nodes are still gliding.
 * 6. The hovered node, else the one whose sheet is open, is focused:
 *    its lines stay bright and the rest are faint (`Links.focus`), and
 *    every node it has no line to is dimmed (`setDim`).
 */
export class Board extends Phaser.Scene {
  private sheet: Sheet | null = null;
  /** Every listed node's brightness, by key: deployed, connected, or mounted into something deployed. */
  private readonly bright = new Map<string, boolean>();
  private ring: Phaser.GameObjects.Arc | null = null;
  private selected: string | null = null;
  private hovered: string | null = null;
  private nodes: Nodes | null = null;
  private links: Links | null = null;
  private edges: Edge[] = [];
  private gliding = false;
  private images = new Map<string, string>();
  private watching: Watching[] = [];

  constructor() {
    super("board");
  }

  create() {
    this.registry.set(STATUS, { kind: "waiting", reason: null } satisfies Status);
    this.registry.set(LINES, { call: true, attachment: true, creator: true } satisfies Lines);
    this.registry.set(KINDS, { account: true, agent: true, standalone_tool: true, dependency_tool: true, volume: true } satisfies Kinds);
    this.scene.launch("ui");
    const show = (_parent: unknown, value: Lines) => {
      this.links?.show(value);
      this.refocus();
    };
    this.registry.events.on(`changedata-${LINES}`, show);
    const shown = (_parent: unknown, value: Kinds) => this.showKinds(value);
    this.registry.events.on(`changedata-${KINDS}`, shown);
    const arranged = () => this.arrange();
    this.game.events.on(ARRANGE, arranged);
    this.events.once("shutdown", () => {
      this.game.events.off(ARRANGE, arranged);
      for (const w of this.watching) w.stop();
      this.registry.events.off(`changedata-${LINES}`, show);
      this.registry.events.off(`changedata-${KINDS}`, shown);
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
    const nodes = makeNodes(this, made.entities, placed.at, () => this.moved(), (entity) => this.select(entity), (node) => this.hover(node), (node) => this.unhover(node));
    this.nodes = nodes;
    this.edges = edges(read, made);
    this.links = makeLinks(this, nodes, this.edges);
    this.links.show(this.registry.get(LINES) as Lines);
    this.showKinds(this.registry.get(KINDS) as Kinds);
    // Rule 3.
    const lit = makeLit(this, nodes);
    // One brightness per node, which the board and the sheet's dot both show.
    const bright = this.bright;
    const shine = (key: string, on: boolean) => {
      bright.set(key, on);
      this.sheet?.told(key, on);
    };
    // A volume is bright while any container it is mounted into is.
    const mounts = new Map<string, string[]>();
    const mountsOfVolume = mountsOf(read);
    const volumeOn = (key: string) => (mounts.get(key) ?? []).some((container) => bright.get(container) === true);
    const relight = () => {
      for (const key of mounts.keys()) {
        const on = volumeOn(key);
        if (bright.get(key) === on) continue;
        lit.set(key, null, on);
        shine(key, on);
      }
    };
    for (const a of read.accounts) shine(accountKey(a), a.connected);
    for (const a of read.agents) shine(agentKey(a), a.deployed);
    for (const t of read.tools) shine(toolKey(t), t.deployed);
    for (const d of read.dependencies) shine(dependencyKey(d), d.deployed);
    for (const v of read.volumes) mounts.set(volumeKey(v), mountsOfVolume(v));
    for (const e of made.entities) if (e.live) lit.set(e.key, e.live, bright.get(e.key));
    for (const key of mounts.keys()) {
      const on = volumeOn(key);
      lit.set(key, null, on);
      shine(key, on);
    }
    this.watching.push(
      watch(
        (key, live, up) => {
          lit.set(key, live, up);
          if (up === undefined) return;
          shine(key, up);
          relight();
        },
        (volume, gone) => {
          const key = volumeKey(volume);
          if (gone) mounts.delete(key);
          else mounts.set(key, mountsOfVolume(volume));
          relight();
        },
      ),
    );
    this.watching.push(pulses(this, nodes, this.links, read.events));
    this.sheet = new Sheet(() => this.deselect(), (key) => bright.get(key) === true);
    // The view's middle may reach the first columns and every node shown wherever it stands now.
    const reach = () => {
      const e = { left: 0, top: 0, right: placed.width, bottom: placed.height };
      for (const node of nodes.byKey.values()) {
        if (!node.image.visible) continue;
        const p = where(node);
        e.left = Math.min(e.left, p.x);
        e.top = Math.min(e.top, p.y);
        e.right = Math.max(e.right, p.x);
        e.bottom = Math.max(e.bottom, p.y);
      }
      return e;
    };
    controls(this, placed.width, placed.height, () => nodes.dragging(), reach);
    const counts: Counts = { account: 0, agent: 0, standalone_tool: 0, dependency_tool: 0, volume: 0, call: 0, attachment: 0, creator: 0 };
    for (const e of made.entities) counts[e.kind]++;
    for (const e of this.edges) counts[e.type]++;
    this.registry.set(STATUS, { kind: "drawn", counts, forbidden: read.forbidden } satisfies Status);
  }

  /** Rule 2: a node moved; its links and its ring follow. */
  private moved() {
    this.links?.redraw();
    const node = this.selected === null ? undefined : this.nodes?.byKey.get(this.selected);
    if (node) this.ring?.setPosition(where(node).x, where(node).y);
  }

  /** Rule 2: show the kinds shown, and hide the rest with their lines; a hidden node's sheet closes. */
  private showKinds(kinds: Kinds) {
    const nodes = this.nodes;
    if (!nodes) return;
    for (const node of nodes.byKey.values()) setShown(node, kinds[node.entity.kind]);
    const selected = this.selected === null ? undefined : nodes.byKey.get(this.selected);
    if (selected && !selected.image.visible) {
      this.sheet?.close();
      this.selected = null;
      this.ring?.setVisible(false);
    }
    const hovered = this.hovered === null ? undefined : nodes.byKey.get(this.hovered);
    if (hovered && !hovered.image.visible) this.hovered = null;
    this.links?.redraw();
    this.refocus();
  }

  /** Rule 5. */
  private arrange() {
    const nodes = this.nodes;
    if (!nodes || nodes.dragging() || this.gliding) return;
    const bodies = [...nodes.byKey.values()].filter((node) => node.image.visible).map((node) => ({ key: node.entity.key, at: where(node), r: Math.max(REACH[node.entity.kind] + 6, node.label.width / 2 + 4) }));
    const to = arrange(bodies, this.edges, this.registry.get(LINES) as Lines);
    const from = new Map(bodies.map((body) => [body.key, body.at]));
    this.gliding = true;
    this.tweens.addCounter({
      from: 0,
      to: 1,
      duration: GLIDE,
      ease: "Cubic.easeInOut",
      onUpdate: (tween) => {
        const t = tween.getValue() ?? 0;
        for (const [key, p] of to) {
          const node = nodes.byKey.get(key);
          const s = from.get(key);
          if (node && s) setAt(node, s.x + (p.x - s.x) * t, s.y + (p.y - s.y) * t);
        }
        this.moved();
      },
      onComplete: () => {
        this.gliding = false;
      },
    });
  }

  /** Rule 6: the pointer came onto a node, so it is focused. */
  private hover(node: Node) {
    if (this.hovered === node.entity.key) return;
    this.hovered = node.entity.key;
    this.refocus();
  }

  /** Rule 6: the pointer left a node. */
  private unhover(node: Node) {
    if (this.hovered !== node.entity.key) return;
    this.hovered = null;
    this.refocus();
  }

  /** Rule 4. */
  private select(entity: Entity) {
    if (this.selected === entity.key) {
      this.sheet?.close();
      this.deselect();
      return;
    }
    const node = this.nodes?.byKey.get(entity.key);
    if (!(entity.subject || entity.files) || !node || !this.ring) return;
    this.selected = entity.key;
    this.ring.setPosition(node.image.x, node.image.y).setRadius(REACH[entity.kind] + 10).setVisible(true);
    this.refocus();
    this.sheet?.open({ entity, images: this.images });
  }

  /** Rule 4: the sheet closed, so nothing is selected. */
  private deselect() {
    this.selected = null;
    this.ring?.setVisible(false);
    this.refocus();
  }

  /** Rule 6. */
  private refocus() {
    const focused = this.hovered ?? this.selected;
    this.links?.focus(focused);
    const nodes = this.nodes;
    if (!nodes) return;
    const near = focused === null ? null : (this.links?.touching(focused) ?? new Set<string>());
    for (const node of nodes.byKey.values()) setDim(node, near !== null && node.entity.key !== focused && !near.has(node.entity.key));
  }
}
