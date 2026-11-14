// The links: every edge a line, the lines between two nodes side by side
// in lanes, each bowed a little, redrawn when a node moves, a type is
// shown or hidden, or a node is focused.

import * as Phaser from "phaser";
import type { Point } from "../world/geometry";
import type { Edge, EdgeType } from "./edges";
import { type Nodes, where } from "./nodes";
import { type Pattern, STYLES } from "./styles";
import { REACH } from "./textures";

/** How far apart side-by-side lanes run, in pixels. */
const GAP = 4.5;

/** How far a lane's bend is pulled off the straight, as a share of its length, and at most, in pixels: it bows half that. */
const BOW = 0.3;
const MOST = 70;

/** How many straight pieces a lane is drawn in. */
const PIECES = 24;

/** A dash and its gap, and a dot and its gap, in pixels. */
const DASH: Record<Exclude<Pattern, "solid">, [number, number]> = { dashed: [6, 4], dotted: [1.5, 3.5] };

/** The chevron's length and half-width, and the maker's dot's radius, in pixels. */
const HEAD = 7;
const WING = 3.5;
const DOT = 2.5;

/** How near in angle, in radians, a maker's lines leave it to share one dot. */
const TOGETHER = 0.6;

/** How much fainter a line not touching the focused node is drawn, as a share of its alpha. */
const UNFOCUSED = 1 / 3;

/** A lane drawn: from where it starts to where it ends, the way it runs, bowed towards `c`. */
export type Lane = { a: Point; c: Point; b: Point };

/** The point `t` of the way along a lane: 0 its start, 1 its end. */
export const pointOn = (l: Lane, t: number): Point => ({
  x: (1 - t) * (1 - t) * l.a.x + 2 * t * (1 - t) * l.c.x + t * t * l.b.x,
  y: (1 - t) * (1 - t) * l.a.y + 2 * t * (1 - t) * l.c.y + t * t * l.b.y,
});

/** A lane as the points it is drawn through, start to end. */
export const traced = (l: Lane): Point[] => Array.from({ length: PIECES + 1 }, (_, i) => pointOn(l, i / PIECES));

/** A run of points' length. */
const lengthOf = (points: Point[]) => points.reduce((sum, p, i) => (i === 0 ? 0 : sum + Math.hypot(p.x - points[i - 1].x, p.y - points[i - 1].y)), 0);

/** A lane's length along its bow. */
export const laneLength = (l: Lane): number => lengthOf(traced(l));

/** The part of a run of points between `from` and `to` along it. */
function between(points: Point[], from: number, to: number): Point[] {
  const out: Point[] = [];
  let walked = 0;
  for (let i = 1; i < points.length; i++) {
    const p = points[i - 1];
    const q = points[i];
    const piece = Math.hypot(q.x - p.x, q.y - p.y);
    const lo = Math.max(from, walked);
    const hi = Math.min(to, walked + piece);
    if (hi >= lo && piece > 0) {
      const at = (d: number) => ({ x: p.x + ((q.x - p.x) * (d - walked)) / piece, y: p.y + ((q.y - p.y) * (d - walked)) / piece });
      if (out.length === 0) out.push(at(lo));
      out.push(at(hi));
    }
    walked += piece;
    if (walked >= to) break;
  }
  return out;
}

/** Points as `Graphics.strokePoints` takes them: it reads only their `x` and `y`. */
export const asPoints = (points: Point[]) => points as Phaser.Math.Vector2[];

/** The part of a lane between `from` and `to` pixels along it. */
export const span = (l: Lane, from: number, to: number): Point[] => between(traced(l), from, to);

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
 *    only the lines shown, between two nodes shown, take a lane, so
 *    hiding one closes the gap.
 * 2. Every lane is measured in its pair's fixed frame, from `a` to `b`,
 *    so dragging never changes the order.
 * 3. Every lane bows: its bend is pulled off the straight by `BOW` of
 *    its length, at most `MOST` pixels, so it bows at most half that —
 *    a call or a creator to the left of `a` to `b`, an attachment to
 *    the right. Lines leaving one node fan apart, and a line between
 *    two nodes in a row passes the nodes between rather than running
 *    under them. It leaves each shape towards its bend and stops short
 *    of it.
 * 4. Every lane is drawn in its type's style (`STYLES`), flat: the
 *    line, then its mark — a call's chevron at the receiver, along the
 *    bow; a creator's open dot at the maker, one dot for all a maker's
 *    lines that leave it within `TOGETHER` of each other.
 * 5. A made-by line (a creator) is shown while its type is, or while
 *    either of its nodes is focused: pointing at a node, or opening it,
 *    shows who made it and what it made.
 * 6. While a node is focused, every line not touching it is drawn
 *    `UNFOCUSED` as bright.
 * 7. They are drawn again only on a move, a show, a focus, or new
 *    edges (`setEdges`), which replace every edge as the world now has
 *    them.
 * 8. A node's touching nodes are those a line shown joins it to, both
 *    ends shown.
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

  /** Rule 5: whether an edge is drawn, its type aside from whether its nodes are. */
  const on = (e: Edge) => visible[e.type] || (e.type === "creator" && focused !== null && (e.a === focused || e.b === focused));

  /** Rules 1 to 3: an edge's lane, the way it runs, or `null` when hidden or too short. */
  const lane = (e: Edge): Lane | null => {
    if (!on(e)) return null;
    const shown = (pairs.get(`${e.a}\n${e.b}`) ?? []).filter(on);
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
    const left = { x: d.y, y: -d.x };
    // Rule 3.
    const pull = Math.min(BOW * length, MOST) * (e.type === "attachment" ? -1 : 1);
    const bend = { x: (p.x + q.x) / 2 + left.x * pull, y: (p.y + q.y) / 2 + left.y * pull };
    const toward = (from: Point, reach: number) => {
      const r = Math.hypot(bend.x - from.x, bend.y - from.y);
      return { x: from.x + ((bend.x - from.x) / r) * reach, y: from.y + ((bend.y - from.y) / r) * reach };
    };
    const offset = (i - (shown.length - 1) / 2) * GAP;
    const n = { x: left.x * offset, y: left.y * offset };
    const move = (at: Point) => ({ x: at.x + n.x, y: at.y + n.y });
    const start = move(toward(p, sa));
    const end = move(toward(q, sb));
    const c = move(bend);
    return e.from === e.b ? { a: end, c, b: start } : { a: start, c, b: end };
  };

  const redraw = () => {
    g.clear();
    /** Rule 4: each maker's dots, by the way its lines leave it. */
    const dots = new Map<string, { color: number; alpha: number; at: Point[] }>();
    for (const e of all) {
      const l = lane(e);
      if (!l) continue;
      const style = STYLES[e.type];
      const from = e.from === undefined ? null : (nodes.byKey.get(e.from)?.entity.kind ?? null);
      const color = style.color(from);
      // Rule 6.
      const alpha = style.alpha * (focused === null || e.a === focused || e.b === focused ? 1 : UNFOCUSED);
      g.lineStyle(style.width, color, alpha);
      stroke(g, traced(l), style.pattern);
      if (style.head === "chevron") chevron(g, l);
      if (style.head === "dot" && e.from !== undefined) {
        const maker = nodes.byKey.get(e.from);
        if (!maker) continue;
        const m = where(maker);
        const way = Math.round(Math.atan2(l.a.y - m.y, l.a.x - m.x) / TOGETHER);
        const id = `${e.from}\n${way}\n${color}\n${alpha}`;
        const group = dots.get(id) ?? { color, alpha, at: [] };
        group.at.push(l.a);
        dots.set(id, group);
      }
    }
    for (const dot of dots.values()) {
      const x = dot.at.reduce((sum, p) => sum + p.x, 0) / dot.at.length;
      const y = dot.at.reduce((sum, p) => sum + p.y, 0) / dot.at.length;
      g.lineStyle(STYLES.creator.width, dot.color, dot.alpha);
      g.strokeCircle(x, y, DOT);
    }
  };
  redraw();
  return {
    redraw,
    // Rule 7.
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
    // Rule 8.
    touching(key) {
      const near = new Set<string>();
      for (const e of all) {
        if ((e.a !== key && e.b !== key) || !(visible[e.type] || (e.type === "creator" && key === focused))) continue;
        const other = e.a === key ? e.b : e.a;
        if (nodes.byKey.get(key)?.image.visible && nodes.byKey.get(other)?.image.visible) near.add(other);
      }
      return near;
    },
  };
}

/** A lane's points in its pattern, in the graphics' line style: dashes and dots run on along the bow. */
function stroke(g: Phaser.GameObjects.Graphics, points: Point[], pattern: Pattern): void {
  if (pattern === "solid") {
    g.strokePoints(asPoints(points), false, false);
    return;
  }
  const [dash, gap] = DASH[pattern];
  const length = lengthOf(points);
  for (let at = 0; at < length; at += dash + gap) {
    const piece = between(points, at, Math.min(at + dash, length));
    if (piece.length > 1) g.strokePoints(asPoints(piece), false, false);
  }
}

/** A chevron at the lane's end, pointing the way the bow arrives, in the graphics' line style. */
function chevron(g: Phaser.GameObjects.Graphics, l: Lane): void {
  const run = Math.hypot(l.b.x - l.c.x, l.b.y - l.c.y);
  if (run <= 0) return;
  const d = { x: (l.b.x - l.c.x) / run, y: (l.b.y - l.c.y) / run };
  const base = { x: l.b.x - d.x * HEAD, y: l.b.y - d.y * HEAD };
  g.lineBetween(l.b.x, l.b.y, base.x + d.y * WING, base.y - d.x * WING);
  g.lineBetween(l.b.x, l.b.y, base.x - d.y * WING, base.y + d.x * WING);
}
