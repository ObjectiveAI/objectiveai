// The board scene: every entity as a node on the theme's grid,
// dragged wherever its owner wants it. It follows the daemon: what is
// made appears, what changes is redrawn, what is deleted goes. One board
// per tab of the window: Home, our own daemon, and each daemon opened
// along a route.

import * as Phaser from "phaser";
import { arrange } from "../grid/arrange";
import { type Edge, edges } from "../grid/edges";
import { type Links, makeLinks } from "../grid/links";
import { type Here, makeHere } from "../grid/here";
import { type Lit, makeLit } from "../grid/live";
import { makeNodes, type Node, type Nodes, setAt, setDim, setShown, where } from "../grid/nodes";
import { place, placeOne } from "../grid/place";
import { type Pulses, pulses } from "../grid/pulses";
import { REACH } from "../grid/textures";
import type { Route } from "../lib/wire";
import { Sheet } from "../sheet/sheet";
import { follow } from "../world/live";
import { accountKey, agentKey, connectedKey, daemonKey, dependencyKey, type Entity, type Model, model, providerKey, providerLive, toolKey, volumeKey } from "../world/model";
import { mountsOf } from "../world/mounts";
import type { Snapshot } from "../world/snapshot";
import type { Watching } from "../world/watch";
import { controls } from "./camera";
import { ACTIVE, ARRANGE, type Bounds, type Counts, FOCUS, HOVER, KINDS, type Kinds, LINES, type Lines, OPEN, type Opening, OUTLINE, OUTLINER, type Outline, SELECTED, STATUS, type Status } from "./status";

/** How long nodes take to glide to their arranged places, in milliseconds. */
const GLIDE = 700;

/** How long the view takes to bring a node the outliner chose to its middle, in milliseconds. */
const PAN = 400;

/**
 * The board: Home, started by the backdrop, which starts the UI scene
 * over itself; or a daemon's, added by the tab strip (`tabs`) with the
 * route to it (`Opening`).
 *
 * The rules:
 *
 * 1. The daemon the board's route reaches is followed (`follow`): its
 *    lists kept open, the world handed on as it changes. Until every
 *    list has been whole once, the board waits.
 * 2. Every entity is a node at its first place (`place`), every edge
 *    (`edges`) — call, attachment, creator — a line under them, side by
 *    side in lanes, each bowed (`makeLinks`). A node dragged takes its
 *    lines, its corners and its pulses with it. The outliner shows and
 *    hides each type of line (`LINES`) and each kind of node (`KINDS`),
 *    made-by lines hidden at first (rule 7 shows a node's own); a hidden node
 *    takes its lines and pulses with it, and closes its sheet if it was
 *    open.
 * 3. Every later world is drawn against the board (`apply`): an entity
 *    new to it is a node at its own place (`placeOne`), one gone from it
 *    is removed with its lines — its sheet closed if it was open — and
 *    one still in it is relabelled; the lines are the world's edges
 *    now. A node that stays keeps where it stands. The outliner's counts
 *    are the world's.
 * 4. Every node is lit as the world says (`light`): bright or faint,
 *    traced while it works; pulses run as the events tell
 *    (`pulses`), after the world that holds their line is drawn.
 * 5. Clicking any node opens its sheet — its tabs, those it has, or
 *    none: a provider's or a daemon's is its header alone — and puts the gold corners round it (`makeHere`); clicking another swaps the sheet and the corners;
 *    clicking the same one again, or the sheet's ✕, closes it.
 * 6. Arrange (`ARRANGE`) gives every node shown a new place by the
 *    lines shown (`arrange`), and the nodes glide there over `GLIDE`, their
 *    lines and corners following. It does nothing while a node is
 *    dragged, or while nodes are still gliding.
 * 7. The hovered node, else the one whose sheet is open, is focused:
 *    its lines stay bright, its made-by lines show — who made it and
 *    what it made — and the rest are faint (`Links.focus`), and every
 *    node it has no line to is dimmed (`setDim`). A row hovered
 *    in the outliner (`HOVER`) focuses its node as hovering it does.
 * 8. The outliner is told every node (`OUTLINE`) with its brightness,
 *    as each world is drawn, and the node whose sheet is open
 *    (`SELECTED`). A row chosen in it (`FOCUS`) brings its node, if
 *    shown, to the view's middle over `PAN`, and opens its sheet as
 *    clicking it would — or keeps it open, if it is open already. The
 *    view neither pans nor zooms for a press or a wheel over the
 *    outliner (`OUTLINER`).
 * 9. Its route (`Opening.route`, `[]` for Home) is on every request it
 *    and its sheet make; a daemon's sheet opens that daemon's tab
 *    (`OPEN`) with the route one hop longer, and its name added.
 * 10. Only the board shown (`ACTIVE`) speaks to the outliner: what it
 *    would write of `STATUS`, `OUTLINE` and `SELECTED` is kept (`say`)
 *    and written only while it is shown, and all of it again when it
 *    wakes; `ARRANGE`, `FOCUS`, `HOVER` and pulses are its alone. Which
 *    lines and kinds are shown (`LINES`, `KINDS`) is every board's
 *    alike, set by the first. Put to sleep, it closes its sheet; its
 *    lists stay open and its world current. Shut down — its tab closed —
 *    its lists close and its sheet leaves the page.
 */
export class Board extends Phaser.Scene {
  private sheet: Sheet | null = null;
  /** Every listed node's brightness, by key: deployed, connected, or mounted into something deployed. */
  private readonly bright = new Map<string, boolean>();
  private here: Here | null = null;
  /** The zoom the nodes' names were last fitted to. */
  private fitted = 0;
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
  /** Rule 9: the route to the daemon this board shows, and each daemon's name along it. */
  private route: Route = [];
  private labels: string[] = [];
  /** Rule 10: the last of each value the board would tell the outliner, by registry key. */
  private readonly said = new Map<string, unknown>();

  constructor(key = "board") {
    super(key);
  }

  /** Rule 9: Home, unless a route is given. */
  init(data: Partial<Opening>) {
    this.route = data.route ?? [];
    this.labels = data.labels ?? [];
  }

  create() {
    this.say(STATUS, { kind: "waiting", reason: null } satisfies Status);
    // Rule 10: set by the first board, shared by the rest.
    if (this.registry.get(LINES) === undefined) this.registry.set(LINES, { call: true, attachment: true, creator: false } satisfies Lines);
    if (this.registry.get(KINDS) === undefined) this.registry.set(KINDS, { account: true, agent: true, standalone_tool: true, connected_standalone_tool: true, dependency_tool: true, volume: true, provider: true, daemon: true } satisfies Kinds);
    this.say(OUTLINE, [] satisfies Outline);
    this.say(SELECTED, null);
    if (this.route.length === 0) this.scene.launch("ui");
    const show = (_parent: unknown, value: Lines) => {
      this.links?.show(value);
      this.refocus();
    };
    this.registry.events.on(`changedata-${LINES}`, show);
    const shown = (_parent: unknown, value: Kinds) => this.showKinds(value);
    this.registry.events.on(`changedata-${KINDS}`, shown);
    // Rule 10: the outliner's words are the shown board's alone.
    const arranged = () => this.awake() && this.arrange();
    this.game.events.on(ARRANGE, arranged);
    const focus = (key: string) => this.awake() && this.focus(key);
    this.game.events.on(FOCUS, focus);
    const hoverKey = (key: string | null) => this.awake() && this.hoverKey(key);
    this.game.events.on(HOVER, hoverKey);
    // Rule 10: asleep, the sheet closed; awake, the outliner told it all again.
    const slept = () => {
      if (this.selected !== null) {
        this.sheet?.close();
        this.deselect();
      }
      this.hovered = null;
      this.refocus();
    };
    this.events.on("sleep", slept);
    const woke = () => {
      for (const [key, value] of this.said) this.registry.set(key, value);
    };
    this.events.on("wake", woke);
    this.events.once("shutdown", () => {
      this.game.events.off(ARRANGE, arranged);
      this.game.events.off(FOCUS, focus);
      this.game.events.off(HOVER, hoverKey);
      this.events.off("sleep", slept);
      this.events.off("wake", woke);
      this.following?.stop();
      this.sheet?.dispose();
      this.sheet = null;
      this.registry.events.off(`changedata-${LINES}`, show);
      this.registry.events.off(`changedata-${KINDS}`, shown);
    });
    // Rules 1 and 9.
    this.following = follow(
      this.route,
      (world) => (this.nodes ? this.apply(world) : this.draw(world)),
      (change) => this.pulses?.told(change),
    );
  }

  /** Rule 10: whether this board is the one shown. */
  private awake(): boolean {
    return this.registry.get(ACTIVE) === this.scene.key;
  }

  /** Rule 10: a value for the outliner, kept, and written while this board is shown. */
  private say(key: string, value: unknown) {
    this.said.set(key, value);
    if (this.awake()) this.registry.set(key, value);
  }

  /** Rule 2: the first world. */
  private draw(read: Snapshot) {
    const made = model(read);
    this.images = made.images;
    const placed = place(made.entities);
    this.here = makeHere(this);
    const nodes = makeNodes(this, made.entities, placed.at, () => this.moved(), (entity) => this.select(entity), (node) => this.hover(node), (node) => this.unhover(node));
    this.nodes = nodes;
    this.edges = edges(read, made);
    this.links = makeLinks(this, nodes, this.edges);
    this.links.show(this.registry.get(LINES) as Lines);
    this.showKinds(this.registry.get(KINDS) as Kinds);
    this.lit = makeLit(this, nodes);
    this.pulses = pulses(this, nodes, this.links, read.events, () => this.awake());
    // Rule 9: a daemon's sheet opens its tab, one hop further along.
    this.sheet = new Sheet(
      () => this.deselect(),
      (key) => this.bright.get(key) === true,
      (id, label) => this.game.events.emit(OPEN, { route: [...this.route, id], labels: [...this.labels, label] } satisfies Opening),
    );
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
      return b !== undefined && (b.held || (x >= b.x && x <= b.x + b.width && y >= b.y && y <= b.y + b.height));
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
   * it; volumes while mounted into something bright; the trace running
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
    this.say(OUTLINE, made.entities.map((e) => ({ key: e.key, kind: e.kind, label: e.label, bright: this.bright.get(e.key) === true })) satisfies Outline);
    const counts: Counts = { account: 0, agent: 0, standalone_tool: 0, connected_standalone_tool: 0, dependency_tool: 0, volume: 0, provider: 0, daemon: 0, call: 0, attachment: 0, creator: 0 };
    for (const e of made.entities) counts[e.kind]++;
    for (const e of this.edges) counts[e.type]++;
    this.say(STATUS, { kind: "drawn", counts, forbidden: read.forbidden } satisfies Status);
  }

  /** Rule 2: a node moved; its links and its corners follow. */
  private moved() {
    this.links?.redraw();
    const node = this.selected === null ? undefined : this.nodes?.byKey.get(this.selected);
    if (node) this.here?.move(where(node).x, where(node).y);
  }

  /** Every frame: when the zoom has changed, each node's name is fitted again, so it stays one size on screen. */
  update() {
    const zoom = this.cameras.main.zoom;
    if (zoom === this.fitted || !this.nodes) return;
    this.fitted = zoom;
    for (const node of this.nodes.byKey.values()) setAt(node, node.image.x, node.image.y);
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
      this.say(SELECTED, null);
      this.here?.hide();
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
    if (!node || !this.here) return;
    this.selected = entity.key;
    this.say(SELECTED, entity.key);
    this.here.show(entity.kind, node.image.x, node.image.y);
    this.refocus();
    this.sheet?.open({ entity, images: this.images, route: this.route });
  }

  /** Rule 5: the sheet closed, so nothing is selected. */
  private deselect() {
    this.selected = null;
    this.say(SELECTED, null);
    this.here?.hide();
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
