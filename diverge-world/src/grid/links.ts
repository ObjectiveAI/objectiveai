// The links: every edge a faint line, the lines between two nodes side
// by side in lanes, redrawn when a node moves, a type is shown or
// hidden, or a node is focused.

import * as Phaser from "phaser";
import type { Point } from "../world/geometry";
import type { Edge, EdgeType } from "./edges";
import { type Nodes, where } from "./nodes";
import { type Pattern, STYLES } from "./styles";
import { REACH } from "./textures";

/** How far apart side-by-side lanes run, in pixels. */
const GAP = 4.5;

/** A dash and its gap, and a dot and its gap, in pixels. */
const DASH: Record<Exclude<Pattern, "solid">, [number, number]> = { dashed: [6, 4], dotted: [1.5, 3.5] };

/** The chevron's length and half-width, and the maker's dot's radius, in pixels. */
const HEAD = 7;
const WING = 3.5;
const DOT = 2.5;

/** How much fainter a line not touching the focused node is drawn, and the glow under each line, as a share of its alpha. */
const UNFOCUSED = 1 / 3;
const GLOW = 0.25;

/** A lane drawn: from where it starts to where it ends, the way it runs. */
export type Lane = { a: Point; b: Point };

/** The links: drawn again on `redraw`, shown by type, focused on one node. */
export type Links = {
  redraw(): void;
  setEdges(next: Edge[]): void;
  show(visible: Record<EdgeType, boolean>): void;
  focus(key: string | null): void;
  laneOf(from: string, to: string, type: EdgeType): Lane | null;
  touching(key: string): Set<string>;
};

/** Where an edge's lane lies among its pair's: calls from `a`, then from `b`, then the attachment, then the creators. */
function rank(e: Edge): number {
  switch (e.type) {
    case "call":
      return e.from === e.a ? 0 : 1;
    case "attachment":
      return 2;
    case "creator":
      return e.from === e.a ? 3 : 4;
  }
}

/**
 * The links over the field and under the nodes.
 *
 * The rules:
 *
 * 1. The edges between two nodes are a pair's lanes, in `rank`'s order,
 *    side by side `GAP` apart and centred on the line between the two;
 *    only the types shown, between two nodes shown, take a lane, so
 *    hiding one closes the gap.
 * 2. Every lane is measured in its pair's fixed frame, from `a` to `b`,
 *    so dragging never changes the order.
 * 3. Every lane stops short of the shapes it joins, and is drawn in its
 *    type's style (`STYLES`): a faint glow under it, then the line, then
 *    its mark — a call's chevron at the receiver, a creator's open dot
 *    at the maker.
 * 4. While a node is focused, every line not touching it is drawn
 *    `UNFOCUSED` as bright.
 * 5. They are drawn again only on a move, a show, a focus, or new
 *    edges (`setEdges`), which replace every edge as the world now has
 *    them.
 * 6. A node's touching nodes are those a line of a type shown joins it
 *    to, both ends shown.
 */
export function makeLinks(scene: Phaser.Scene, nodes: Nodes, first: Edge[]): Links {
  let all = first;
  const g = scene.add.graphics().setDepth(3);
  const pairs = new Map<string, Edge[]>();
  const pair = () => {
    pairs.clear();
    for (const e of all) {
      const id = `${e.a}\n${e.b}`;
      const list = pairs.get(id) ?? [];
      list.push(e);
      pairs.set(id, list);
    }
    for (const list of pairs.values()) list.sort((x, y) => rank(x) - rank(y));
  };
  pair();
  let visible: Record<EdgeType, boolean> = { call: true, attachment: true, creator: true };
  let focused: string | null = null;

  /** Rules 1 to 3: an edge's lane, the way it runs, or `null` when hidden or too short. */
  const lane = (e: Edge): Lane | null => {
    if (!visible[e.type]) return null;
    const shown = (pairs.get(`${e.a}\n${e.b}`) ?? []).filter((x) => visible[x.type]);
    const i = shown.indexOf(e);
    const A = nodes.byKey.get(e.a);
    const B = nodes.byKey.get(e.b);
    if (i < 0 || !A || !B || !A.image.visible || !B.image.visible) return null;
    const p = where(A);
    const q = where(B);
    const length = Math.hypot(q.x - p.x, q.y - p.y);
    const sa = REACH[A.entity.kind] + 5;
    const sb = REACH[B.entity.kind] + 5;
    if (length <= sa + sb) return null;
    const d = { x: (q.x - p.x) / length, y: (q.y - p.y) / length };
    const offset = (i - (shown.length - 1) / 2) * GAP;
    const n = { x: d.y * offset, y: -d.x * offset };
    const start = { x: p.x + d.x * sa + n.x, y: p.y + d.y * sa + n.y };
    const end = { x: q.x - d.x * sb + n.x, y: q.y - d.y * sb + n.y };
    return e.from === e.b ? { a: end, b: start } : { a: start, b: end };
  };

  const redraw = () => {
    g.clear();
    for (const e of all) {
      const l = lane(e);
      if (!l) continue;
      const style = STYLES[e.type];
      const from = e.from === undefined ? null : (nodes.byKey.get(e.from)?.entity.kind ?? null);
      const color = style.color(from);
      // Rule 4.
      const alpha = style.alpha * (focused === null || e.a === focused || e.b === focused ? 1 : UNFOCUSED);
      // Rule 3.
      g.lineStyle(style.width + 3, color, alpha * GLOW);
      stroke(g, l, style.pattern);
      g.lineStyle(style.width, color, alpha);
      stroke(g, l, style.pattern);
      if (style.head === "chevron") chevron(g, l);
      if (style.head === "dot") g.strokeCircle(l.a.x, l.a.y, DOT);
    }
  };
  redraw();
  return {
    redraw,
    // Rule 5.
    setEdges(next) {
      all = next;
      pair();
      redraw();
    },
    show(next) {
      visible = { ...next };
      redraw();
    },
    focus(key) {
      focused = key;
      redraw();
    },
    laneOf(from, to, type) {
      const [a, b] = from < to ? [from, to] : [to, from];
      const e = (pairs.get(`${a}\n${b}`) ?? []).find((x) => x.type === type && (x.from === undefined || x.from === from));
      return e ? lane(e) : null;
    },
    // Rule 6.
    touching(key) {
      const near = new Set<string>();
      for (const e of all) {
        if (!visible[e.type] || (e.a !== key && e.b !== key)) continue;
        const other = e.a === key ? e.b : e.a;
        if (nodes.byKey.get(key)?.image.visible && nodes.byKey.get(other)?.image.visible) near.add(other);
      }
      return near;
    },
  };
}

/** A lane in its pattern, in the graphics' line style. */
function stroke(g: Phaser.GameObjects.Graphics, l: Lane, pattern: Pattern): void {
  if (pattern === "solid") {
    g.lineBetween(l.a.x, l.a.y, l.b.x, l.b.y);
    return;
  }
  const [dash, gap] = DASH[pattern];
  const length = Math.hypot(l.b.x - l.a.x, l.b.y - l.a.y);
  const d = { x: (l.b.x - l.a.x) / length, y: (l.b.y - l.a.y) / length };
  for (let at = 0; at < length; at += dash + gap) {
    const end = Math.min(at + dash, length);
    g.lineBetween(l.a.x + d.x * at, l.a.y + d.y * at, l.a.x + d.x * end, l.a.y + d.y * end);
  }
}

/** A chevron at the lane's end, in the graphics' line style. */
function chevron(g: Phaser.GameObjects.Graphics, l: Lane): void {
  const length = Math.hypot(l.b.x - l.a.x, l.b.y - l.a.y);
  const d = { x: (l.b.x - l.a.x) / length, y: (l.b.y - l.a.y) / length };
  const base = { x: l.b.x - d.x * HEAD, y: l.b.y - d.y * HEAD };
  g.lineBetween(l.b.x, l.b.y, base.x + d.y * WING, base.y - d.x * WING);
  g.lineBetween(l.b.x, l.b.y, base.x - d.y * WING, base.y + d.x * WING);
}
