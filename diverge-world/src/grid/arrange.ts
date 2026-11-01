// Arranging the board: new places for every node, so the lines read as
// cleanly as they can.

import type { Point } from "../world/geometry";
import { generator, hash } from "../world/random";
import type { Edge, EdgeType } from "./edges";

/** One node to arrange: its key, where it stands now, and the room it takes — its shape and its label. */
export type Body = { key: string; at: Point; r: number };

/** How hard each type of line pulls its two ends together. */
const PULL: Record<EdgeType, number> = { call: 1, attachment: 1, creator: 0.35 };

/** The distance a line would like to be, in pixels. */
const IDEAL = 150;

/** The force pass's steps, its first step's reach, and how hard every node is drawn to the middle. */
const FORCE_STEPS = 400;
const FORCE_REACH = 90;
const GRAVITY = 0.04;

/** The clean-up pass's tries, and its first move's reach. */
const TRIES = 4000;
const MOVE = 140;

/** What each fault costs. */
const CROSSING = 10;
const THROUGH = 8;
const OVERLAP = 6;
const FAN = 2;
const LONG = 0.05;

/** Two lines leaving one node closer than this, in radians, fan. */
const NARROW = 0.35;

/** A line's two ends, by index. */
type Join = { i: number; j: number; w: number };

const dist = (p: Point, q: Point) => Math.hypot(q.x - p.x, q.y - p.y);

/** Whether the segments `a`–`b` and `c`–`d` cross, not merely touch. */
function crosses(a: Point, b: Point, c: Point, d: Point): boolean {
  const side = (p: Point, q: Point, r: Point) => (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
  const d1 = side(c, d, a);
  const d2 = side(c, d, b);
  const d3 = side(a, b, c);
  const d4 = side(a, b, d);
  return d1 * d2 < 0 && d3 * d4 < 0;
}

/** The distance from `p` to the segment `a`–`b`. */
function toSegment(p: Point, a: Point, b: Point): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy || 1)));
  return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
}

/**
 * How unclean a placing is: the sum of its faults.
 *
 * 1. Two lines that cross, not sharing an end: `CROSSING`.
 * 2. A line passing through a node not its own end: up to `THROUGH`,
 *    the nearer the more.
 * 3. Two nodes whose room overlaps, labels included: up to `OVERLAP`.
 * 4. Two lines leaving one node within `NARROW` of each other: up to
 *    `FAN`.
 * 5. Every line longer than `IDEAL`: `LONG` for each `IDEAL` beyond,
 *    squared, so the board stays together.
 */
function cost(at: Point[], r: number[], joins: Join[]): number {
  let c = 0;
  // Rule 1.
  for (let x = 0; x < joins.length; x++) {
    const p = joins[x];
    for (let y = x + 1; y < joins.length; y++) {
      const q = joins[y];
      if (p.i === q.i || p.i === q.j || p.j === q.i || p.j === q.j) continue;
      if (crosses(at[p.i], at[p.j], at[q.i], at[q.j])) c += CROSSING;
    }
  }
  // Rule 2.
  for (const p of joins) {
    for (let n = 0; n < at.length; n++) {
      if (n === p.i || n === p.j) continue;
      const room = r[n] + 6;
      const d = toSegment(at[n], at[p.i], at[p.j]);
      if (d < room) c += THROUGH * (1 - d / room);
    }
  }
  // Rule 3.
  for (let m = 0; m < at.length; m++) {
    for (let n = m + 1; n < at.length; n++) {
      const room = r[m] + r[n] + 12;
      const d = dist(at[m], at[n]);
      if (d < room) c += OVERLAP * (1 - d / room);
    }
  }
  // Rule 4.
  const leaving: number[][] = at.map(() => []);
  for (const p of joins) {
    leaving[p.i].push(Math.atan2(at[p.j].y - at[p.i].y, at[p.j].x - at[p.i].x));
    leaving[p.j].push(Math.atan2(at[p.i].y - at[p.j].y, at[p.i].x - at[p.j].x));
  }
  for (const angles of leaving) {
    if (angles.length < 2) continue;
    angles.sort((a, b) => a - b);
    for (let k = 0; k < angles.length; k++) {
      const next = k + 1 < angles.length ? angles[k + 1] : angles[0] + 2 * Math.PI;
      const gap = next - angles[k];
      if (gap < NARROW) c += FAN * (1 - gap / NARROW);
    }
  }
  // Rule 5.
  for (const p of joins) {
    const beyond = dist(at[p.i], at[p.j]) / IDEAL - 1;
    if (beyond > 0) c += LONG * beyond * beyond;
  }
  return c;
}

/**
 * New places for every body.
 *
 * The rules:
 *
 * 1. Only the lines shown count; the lines between two nodes count as
 *    one join, pulling as hard as their types together (`PULL`).
 * 2. First, forces, from where the nodes stand now: every two nodes
 *    push apart, every join pulls its two ends towards `IDEAL` apart,
 *    and every node is drawn a little to the middle, so groups with no
 *    line between them stay near. Each step moves a node less than the
 *    one before.
 * 3. Then the clean-up: `TRIES` times, one node is moved a little —
 *    less as the tries go on — or two nodes swap places, and the change
 *    is kept when the placing costs less (`cost`), or now and then when
 *    it costs a little more, less often as the tries go on.
 * 4. The chance is drawn from the nodes' keys, so the same board with
 *    the same lines arranges the same way.
 * 5. The arrangement keeps the middle where the nodes' middle was.
 */
export function arrange(bodies: Body[], edges: Edge[], shown: Record<EdgeType, boolean>): Map<string, Point> {
  const index = new Map(bodies.map((b, i) => [b.key, i]));
  const r = bodies.map((b) => b.r);
  // Rule 1.
  const weights = new Map<string, Join>();
  for (const e of edges) {
    if (!shown[e.type]) continue;
    const i = index.get(e.a);
    const j = index.get(e.b);
    if (i === undefined || j === undefined || i === j) continue;
    const id = `${i}\n${j}`;
    const join = weights.get(id) ?? { i, j, w: 0 };
    join.w += PULL[e.type];
    weights.set(id, join);
  }
  const joins = [...weights.values()];
  const n = bodies.length;
  const middle = bodies.reduce((m, b) => ({ x: m.x + b.at.x / n, y: m.y + b.at.y / n }), { x: 0, y: 0 });
  // Rule 4.
  const random = generator(hash(bodies.map((b) => b.key).sort().join("\n")));
  // Rule 2. A node standing on another is nudged off it first, so every push has a direction.
  const at = bodies.map((b) => ({ x: b.at.x + (random() - 0.5), y: b.at.y + (random() - 0.5) }));
  for (let step = 0; step < FORCE_STEPS; step++) {
    const reach = FORCE_REACH * (1 - step / FORCE_STEPS) + 1;
    const move = at.map(() => ({ x: 0, y: 0 }));
    for (let a = 0; a < n; a++) {
      for (let b = a + 1; b < n; b++) {
        const dx = at[a].x - at[b].x;
        const dy = at[a].y - at[b].y;
        const d = Math.max(1, Math.hypot(dx, dy));
        const push = (IDEAL * IDEAL) / d;
        move[a].x += (dx / d) * push;
        move[a].y += (dy / d) * push;
        move[b].x -= (dx / d) * push;
        move[b].y -= (dy / d) * push;
      }
    }
    for (const { i, j, w } of joins) {
      const dx = at[j].x - at[i].x;
      const dy = at[j].y - at[i].y;
      const d = Math.max(1, Math.hypot(dx, dy));
      const pull = (w * d * d) / IDEAL;
      move[i].x += (dx / d) * pull;
      move[i].y += (dy / d) * pull;
      move[j].x -= (dx / d) * pull;
      move[j].y -= (dy / d) * pull;
    }
    const c = at.reduce((m, p) => ({ x: m.x + p.x / n, y: m.y + p.y / n }), { x: 0, y: 0 });
    for (let k = 0; k < n; k++) {
      move[k].x += (c.x - at[k].x) * GRAVITY * IDEAL * 0.1;
      move[k].y += (c.y - at[k].y) * GRAVITY * IDEAL * 0.1;
      const length = Math.hypot(move[k].x, move[k].y);
      if (length === 0) continue;
      const by = Math.min(length, reach);
      at[k].x += (move[k].x / length) * by;
      at[k].y += (move[k].y / length) * by;
    }
  }
  // Rule 3.
  let now = cost(at, r, joins);
  for (let t = 0; t < TRIES && n > 1; t++) {
    const heat = 1 - t / TRIES;
    const a = Math.floor(random() * n);
    const before = { ...at[a] };
    let b = -1;
    if (random() < 0.2) {
      b = Math.floor(random() * n);
      if (b === a) continue;
      at[a] = { ...at[b] };
      at[b] = before;
    } else {
      const angle = random() * 2 * Math.PI;
      const by = (MOVE * heat + 8) * random();
      at[a] = { x: before.x + Math.cos(angle) * by, y: before.y + Math.sin(angle) * by };
    }
    const next = cost(at, r, joins);
    if (next <= now || random() < Math.exp((now - next) / (2 * heat + 0.01))) {
      now = next;
      continue;
    }
    if (b >= 0) {
      at[b] = { ...at[a] };
    }
    at[a] = before;
  }
  // Rule 5.
  const after = at.reduce((m, p) => ({ x: m.x + p.x / n, y: m.y + p.y / n }), { x: 0, y: 0 });
  return new Map(bodies.map((body, k) => [body.key, { x: at[k].x - after.x + middle.x, y: at[k].y - after.y + middle.y }]));
}
