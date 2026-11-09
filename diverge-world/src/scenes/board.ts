// The board scene: every entity as a glowing node on the dark grid,
// dragged wherever its owner wants it. It follows the daemon: what is
// made appears, what changes is redrawn, what is deleted goes.

import * as Phaser from "phaser";
import { arrange } from "../grid/arrange";
import { type Edge, edges } from "../grid/edges";
import { type Links, makeLinks } from "../grid/links";
import { type Lit, makeLit } from "../grid/live";
import { makeNodes, type Node, type Nodes, setAt, setDim, setShown, where } from "../grid/nodes";
import { place, placeOne } from "../grid/place";
import { type Pulses, pulses } from "../grid/pulses";
import { REACH } from "../grid/textures";
import { Sheet } from "../sheet/sheet";
import { follow } from "../world/live";
import { accountKey, agentKey, connectedKey, daemonKey, dependencyKey, type Entity, type Model, model, providerKey, providerLive, toolKey, volumeKey } from "../world/model";
import { mountsOf } from "../world/mounts";
import type { Snapshot } from "../world/snapshot";
import type { Watching } from "../world/watch";
import { controls } from "./camera";
import { ARRANGE, type Bounds, type Counts, FOCUS, HOVER, KINDS, type Kinds, LINES, type Lines, OUTLINE, OUTLINER, type Outline, SELECTED, STATUS, type Status } from "./status";

/** The selection ring's colour: white light. */
const RING = 0xffffff;

/** How long nodes take to glide to their arranged places, in milliseconds. */
const GLIDE = 700;

/** How long the view takes to bring a node the outliner chose to its middle, in milliseconds. */
const PAN = 400;

/**
 * The board, started by the backdrop; it starts the UI scene over
 * itself.
 *
 * The rules:
 *
 * 1. The daemon is followed (`follow`): its eight lists kept open, the
 *    world handed on as it changes. Until every list has been whole
 *    once, the board waits.
 * 2. Every entity is a node at its first place (`place`), every edge
 *    (`edges`) — call, attachment, creator — a line under them, side by
 *    side in lanes (`makeLinks`). A node dragged takes its lines, its
 *    ring and its pulses with it. The outliner shows and hides each type
 *    of line (`LINES`) and each kind of node (`KINDS`); a hidden node
 *    takes its lines and pulses with it, and closes its sheet if it was
 *    open.
 * 3. Every later world is drawn against the board (`apply`): an entity
 *    new to it is a node at its own place (`placeOne`), one gone from it
 *    is removed with its lines — its sheet closed if it was open — and
 *    one still in it is relabelled; the lines are the world's edges
 *    now. A node that stays keeps where it stands. The outliner's counts
 *    are the world's.
 * 4. Every node is lit as the world says (`light`): bright or faint,
 *    its halo breathing while it works; pulses run as the events tell
 *    (`pulses`), after the world that holds their line is drawn.
 * 5. Clicking an agent, a tool, an account or a volume opens its sheet —
 *    its log, its files, or both — and rings it; clicking another swaps the sheet and the ring;
 *    clicking the same one again, or the sheet's ✕, closes it.
 * 6. Arrange (`ARRANGE`) gives every node shown a new place by the
 *    lines shown (`arrange`), and the nodes glide there over `GLIDE`, their
 *    lines and ring following. It does nothing while a node is
 *    dragged, or while nodes are still gliding.
 * 7. The hovered node, else the one whose sheet is open, is focused:
 *    its lines stay bright and the rest are faint (`Links.focus`), and
 *    every node it has no line to is dimmed (`setDim`). A row hovered
 *    in the outliner (`HOVER`) focuses its node as hovering it does.
 * 8. The outliner is told every node (`OUTLINE`) with its brightness,
 *    as each world is drawn, and the node whose sheet is open
 *    (`SELECTED`). A row chosen in it (`FOCUS`) brings its node, if
 *    shown, to the view's middle over `PAN`, and opens its sheet as
 *    clicking it would — or keeps it open, if it is open already. The
 *    view neither pans nor zooms for a press or a wheel over the
 *    outliner (`OUTLINER`).
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
  private following: Watching | null = null;
  private lit: Lit | null = null;
  private pulses: Pulses | null = null;
  /** What each node was last lit with, so a world that changes nothing about it relights nothing. */
  private readonly litWith = new Map<string, string>();

  constructor() {
    super("board");
  }

  create() {
    this.registry.set(STATUS, { kind: "waiting", reason: null } satisfies Status);
    this.registry.set(LINES, { call: true, attachment: true, creator: true } satisfies Lines);
    this.registry.set(KINDS, { account: true, agent: true, standalone_tool: true, connected_standalone_tool: true, dependency_tool: true, volume: true, provider: true, daemon: true } satisfies Kinds);
    this.registry.set(OUTLINE, [] satisfies Outline);
    this.registry.set(SELECTED, null);
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
    const focus = (key: string) => this.focus(key);
    this.game.events.on(FOCUS, focus);
    const hoverKey = (key: string | null) => this.hoverKey(key);
    this.game.events.on(HOVER, hoverKey);
    this.events.once("shutdown", () => {
      this.game.events.off(ARRANGE, arranged);
      this.game.events.off(FOCUS, focus);
      this.game.events.off(HOVER, hoverKey);
      this.following?.stop();
      this.registry.events.off(`changedata-${LINES}`, show);
      this.registry.events.off(`changedata-${KINDS}`, shown);
    });
    // Rule 1.
    this.following = follow(
      (world) => (this.nodes ? this.apply(world) : this.draw(world)),
      (change) => this.pulses?.told(change),
    );
  }

  /** Rule 2: the first world. */
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
    this.lit = makeLit(this, nodes);
    this.pulses = pulses(this, nodes, this.links, read.events);
    this.sheet = new Sheet(() => this.deselect(), (key) => this.bright.get(key) === true);
    this.light(read, made);
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
    // Rule 8: a press or a wheel over the outliner is the outliner's.
    const blocks = (x: number, y: number) => {
      const b = this.registry.get(OUTLINER) as Bounds | undefined;
      return b !== undefined && x >= b.x && x <= b.x + b.width && y >= b.y && y <= b.y + b.height;
    };
    controls(this, placed.width, placed.height, () => nodes.dragging(), reach, blocks);
    this.count(read, made);
  }

  /** Rule 3: a later world, drawn against the board. */
  private apply(read: Snapshot) {
    const nodes = this.nodes;
    const links = this.links;
    if (!nodes || !links) return;
    const made = model(read);
    this.images = made.images;
    const now = new Map(made.entities.map((entity) => [entity.key, entity]));
    // Gone: its sheet closed, its focus let go, its light forgotten, the node removed.
    for (const key of [...nodes.byKey.keys()]) {
      if (now.has(key)) continue;
      if (this.selected === key) {
        this.sheet?.close();
        this.deselect();
      }
      if (this.hovered === key) this.hovered = null;
      this.lit?.forget(key);
      this.litWith.delete(key);
      this.bright.delete(key);
      nodes.remove(key);
    }
    // New: a node at its own place, shown as its kind is; still there: relabelled.
    const kinds = this.registry.get(KINDS) as Kinds;
    const standing = [...nodes.byKey.values()].map((node) => ({ kind: node.entity.kind, at: where(node) }));
    for (const entity of made.entities) {
      if (nodes.byKey.has(entity.key)) {
        nodes.update(entity);
        continue;
      }
      const at = placeOne(entity.kind, standing);
      standing.push({ kind: entity.kind, at });
      setShown(nodes.add(entity, at), kinds[entity.kind]);
    }
    this.edges = edges(read, made);
    links.setEdges(this.edges);
    this.light(read, made);
    this.moved();
    this.refocus();
    this.count(read, made);
  }

  /**
   * Rule 4: every node lit as the world says — accounts bright while
   * connected; agents, tools and dependency tools while deployed, a
   * connected tool while the daemon holds its connect, a provider
   * while connected, a daemon while the daemon is connected to
   * it; volumes while mounted into something bright; the halo breathing
   * while it works — and the open sheet's dot told. A node whose light
   * did not change is not lit again, so its breath runs on.
   */
  private light(read: Snapshot, made: Model) {
    const lit = this.lit;
    if (!lit) return;
    const bright = new Map<string, boolean>();
    for (const a of read.accounts) bright.set(accountKey(a), a.connected);
    for (const a of read.agents) bright.set(agentKey(a), a.deployed);
    for (const t of read.tools) bright.set(toolKey(t), t.deployed);
    for (const t of read.connectedTools) bright.set(connectedKey(t), t.deployed);
    for (const d of read.dependencies) bright.set(dependencyKey(d), d.deployed);
    for (const p of read.providers) bright.set(providerKey(p), providerLive(p).on);
    for (const d of read.daemons) bright.set(daemonKey(d), d.connected);
    const mountsOfVolume = mountsOf(read);
    for (const v of read.volumes) bright.set(volumeKey(v), mountsOfVolume(v).some((container) => bright.get(container) === true));
    for (const entity of made.entities) {
      const on = bright.get(entity.key);
      const signature = JSON.stringify([entity.live?.kind ?? null, entity.live?.on ?? null, on ?? null]);
      if (this.litWith.get(entity.key) === signature) continue;
      this.litWith.set(entity.key, signature);
      lit.set(entity.key, entity.live ?? null, on);
      if (on !== undefined && this.bright.get(entity.key) !== on) {
        this.bright.set(entity.key, on);
        this.sheet?.told(entity.key, on);
      }
    }
  }

  /** Rule 3: the outliner's counts, as the world has them; rule 8: the outliner's nodes. */
  private count(read: Snapshot, made: Model) {
    this.registry.set(OUTLINE, made.entities.map((e) => ({ key: e.key, kind: e.kind, label: e.label, bright: this.bright.get(e.key) === true })) satisfies Outline);
    const counts: Counts = { account: 0, agent: 0, standalone_tool: 0, connected_standalone_tool: 0, dependency_tool: 0, volume: 0, provider: 0, daemon: 0, call: 0, attachment: 0, creator: 0 };
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
      this.registry.set(SELECTED, null);
      this.ring?.setVisible(false);
    }
    const hovered = this.hovered === null ? undefined : nodes.byKey.get(this.hovered);
    if (hovered && !hovered.image.visible) this.hovered = null;
    this.links?.redraw();
    this.refocus();
  }

  /** Rule 6. */
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

  /** Rule 7: the pointer came onto a node, so it is focused. */
  private hover(node: Node) {
    if (this.hovered === node.entity.key) return;
    this.hovered = node.entity.key;
    this.refocus();
  }

  /** Rule 7: the pointer left a node. */
  private unhover(node: Node) {
    if (this.hovered !== node.entity.key) return;
    this.hovered = null;
    this.refocus();
  }

  /** Rule 5. */
  private select(entity: Entity) {
    if (this.selected === entity.key) {
      this.sheet?.close();
      this.deselect();
      return;
    }
    const node = this.nodes?.byKey.get(entity.key);
    if (!(entity.subject || entity.files) || !node || !this.ring) return;
    this.selected = entity.key;
    this.registry.set(SELECTED, entity.key);
    this.ring.setPosition(node.image.x, node.image.y).setRadius(REACH[entity.kind] + 10).setVisible(true);
    this.refocus();
    this.sheet?.open({ entity, images: this.images });
  }

  /** Rule 5: the sheet closed, so nothing is selected. */
  private deselect() {
    this.selected = null;
    this.registry.set(SELECTED, null);
    this.ring?.setVisible(false);
    this.refocus();
  }

  /** Rule 8: a row chosen in the outliner. */
  private focus(key: string) {
    const node = this.nodes?.byKey.get(key);
    if (!node || !node.image.visible) return;
    const at = where(node);
    this.cameras.main.pan(at.x, at.y, PAN, "Cubic.easeInOut");
    if (this.selected !== key) this.select(node.entity);
  }

  /** Rule 7: a row hovered in the outliner, or left. */
  private hoverKey(key: string | null) {
    const node = key === null ? undefined : this.nodes?.byKey.get(key);
    const next = node?.image.visible ? key : null;
    if (this.hovered === next) return;
    this.hovered = next;
    this.refocus();
  }

  /** Rule 7. */
  private refocus() {
    const focused = this.hovered ?? this.selected;
    this.links?.focus(focused);
    const nodes = this.nodes;
    if (!nodes) return;
    const near = focused === null ? null : (this.links?.touching(focused) ?? new Set<string>());
    for (const node of nodes.byKey.values()) setDim(node, near !== null && node.entity.key !== focused && !near.has(node.entity.key));
  }
}
