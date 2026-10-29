// Where each entity of one island goes: the placement that keeps its
// lines out of each other's way.

import { angleBetween, crosses, distanceToSegment, type Point } from "./geometry";
import { centre, distance, type Hex, STEPS } from "./hex";
import type { Pair } from "./model";
import { generator } from "./random";

/** Hexes from one slot to the next: two empty hexes between any two neighbouring entities. */
export const PITCH = 3;

/** What each fault costs. */
const COST = {
  /** A line over another entity's hex. */
  through: 500,
  /** A line over another entity's name plate. */
  plate: 150,
  /** A line over its own end's name plate. */
  ownPlate: 60,
  /** Two lines crossing. */
  crossing: 100,
  /** Two lines leaving one entity too close together. */
  fan: 40,
  /** Every hex width of every line's length. */
  length: 1,
  /** Every hex of the island's bounding box. */
  area: 0.2,
};

/** One hex's width, for hexes whose corners are `1` from their centres. */
const WIDTH = Math.sqrt(3);
/** A line nearer than this to an entity's centre is over its hex. */
const NEAR = 0.55 * WIDTH;
/** Where a name plate is, beneath its entity's centre: two points, its left and its right. */
const PLATE: Point[] = [
  { x: -0.45, y: 1 },
  { x: 0.45, y: 1 },
];
/** A line nearer than this to a plate's point is over the plate. */
const PLATE_NEAR = 0.3;
/** Two lines leaving one entity closer than this, in radians, fan into one another. */
const FAN = (20 * Math.PI) / 180;

/**
 * The hex of each of one island's entities.
 *
 * The rules:
 *
 * 1. Entities sit on slots `PITCH` hexes apart: the hexes `(3i, 3j)`,
 *    a lattice of triangles, so every slot has six neighbouring slots
 *    and two entities in neighbouring slots have two empty hexes
 *    between them. Every hex within two of an entity but itself is
 *    empty land.
 * 2. An island's land is only the hexes within two of its entities,
 *    which join only between neighbouring slots — slots any further
 *    apart are six hexes or more, and their lands leave sea between; so an island's entities always hold
 *    slots that join, neighbour to neighbour, into one group. A move
 *    that would split them is never made.
 * 3. The slots are those within a radius of the middle slot, about
 *    twice as many as there are entities, so there is room to move.
 * 4. The first placement is greedy: breadth first from the entity with
 *    the most pairs (then the first key), each entity on the free slot
 *    beside one already taken that is nearest its placed neighbours.
 * 5. The placement is then improved by simulated annealing: a move
 *    takes one entity to another slot, swapping with the entity there
 *    if there is one, and is kept if it lowers the cost, or by chance
 *    that shrinks as the search cools. The best placement seen is the
 *    answer.
 * 6. The cost is the sum, heaviest first, of: each line over another
 *    entity's hex; each line over a name plate; each two lines that
 *    cross; each two lines leaving one entity less than 20° apart; the
 *    length of every line; and the area of the island. See `COST`.
 * 7. Chance is seeded with `seed`, so the same island always lands the
 *    same way.
 */
export function place(keys: string[], pairs: Pair[], seed: number): Map<string, Hex> {
  const n = keys.length;
  if (n === 1) return new Map([[keys[0], { q: 0, r: 0 }]]);

  const index = new Map(keys.map((k, i) => [k, i]));
  const edges: [number, number][] = pairs.filter((p) => index.has(p.a) && index.has(p.b)).map((p) => [index.get(p.a)!, index.get(p.b)!]);
  const incident: number[][] = keys.map(() => []);
  edges.forEach(([a, b], e) => {
    incident[a].push(e);
    incident[b].push(e);
  });

  // Rules 1 and 3: the slots, in lattice steps, within `radius` of the middle.
  let radius = 1;
  while (1 + 3 * radius * (radius + 1) < 2 * n) radius++;
  const lattice: Hex[] = [];
  for (let i = -radius; i <= radius; i++) {
    for (let j = -radius; j <= radius; j++) if (distance({ q: i, r: j }, { q: 0, r: 0 }) <= radius) lattice.push({ q: i, r: j });
  }
  const slotOf = new Map(lattice.map((s, k) => [`${s.q},${s.r}`, k]));
  const beside: number[][] = lattice.map((s) => STEPS.map((d) => slotOf.get(`${s.q + d.q},${s.r + d.r}`)).filter((k): k is number => k !== undefined));
  const points: Point[] = lattice.map((s) => centre({ q: PITCH * s.q, r: PITCH * s.r }));
  const slots = lattice.length;
  const pos = new Int32Array(n).fill(-1);
  const occupant = new Int32Array(slots).fill(-1);
  const at = (i: number): Point => points[pos[i]];

  // Rule 2: whether the taken slots join into one group.
  const joined = (): boolean => {
    const seen = new Uint8Array(slots);
    const queue = [pos[0]];
    seen[pos[0]] = 1;
    let reached = 0;
    while (queue.length > 0) {
      const s = queue.pop()!;
      reached++;
      for (const t of beside[s]) {
        if (seen[t] === 0 && occupant[t] >= 0) {
          seen[t] = 1;
          queue.push(t);
        }
      }
    }
    return reached === n;
  };

  // Rule 4: breadth first from the most joined, each beside the taken slots, near its placed neighbours.
  const neighbours: number[][] = keys.map((_, i) => incident[i].map((e) => (edges[e][0] === i ? edges[e][1] : edges[e][0])));
  const first = keys.reduce((best, _, i) => (neighbours[i].length > neighbours[best].length ? i : best), 0);
  const order: number[] = [];
  const queued = new Set([first]);
  for (let q = [first]; q.length > 0; ) {
    const i = q.shift()!;
    order.push(i);
    for (const j of neighbours[i]) {
      if (!queued.has(j)) {
        queued.add(j);
        q.push(j);
      }
    }
  }
  for (let i = 0; i < n; i++) if (!queued.has(i)) order.push(i);
  const middle = slotOf.get("0,0")!;
  for (const i of order) {
    const placed = neighbours[i].filter((j) => pos[j] >= 0).map(at);
    let best = -1;
    let bestScore = Infinity;
    for (let s = 0; s < slots; s++) {
      if (occupant[s] >= 0) continue;
      if (i !== order[0] && !beside[s].some((t) => occupant[t] >= 0)) continue;
      if (i === order[0] && s !== middle) continue;
      const p = points[s];
      const score = placed.reduce((sum, q) => sum + Math.hypot(p.x - q.x, p.y - q.y), 0) + 0.01 * Math.hypot(p.x, p.y);
      if (score < bestScore) {
        bestScore = score;
        best = s;
      }
    }
    pos[i] = best;
    occupant[best] = i;
  }

  // Rule 6: the cost, a term at a time.
  const overCost = (pa: Point, pb: Point, k: number, own: boolean): number => {
    const pk = at(k);
    let cost = 0;
    if (!own && distanceToSegment(pk, pa, pb) < NEAR) cost += COST.through;
    if (PLATE.some((o) => distanceToSegment({ x: pk.x + o.x, y: pk.y + o.y }, pa, pb) < PLATE_NEAR)) cost += own ? COST.ownPlate : COST.plate;
    return cost;
  };
  /** What a line costs on its own: its length, and what it passes over. */
  const lineCost = (e: number): number => {
    const [a, b] = edges[e];
    const pa = at(a);
    const pb = at(b);
    let cost = (COST.length * Math.hypot(pb.x - pa.x, pb.y - pa.y)) / WIDTH;
    for (let k = 0; k < n; k++) cost += overCost(pa, pb, k, k === a || k === b);
    return cost;
  };
  /** What a line costs for passing over entity `k`, which is not one of its ends. */
  const lineOver = (e: number, k: number): number => {
    const [a, b] = edges[e];
    return k === a || k === b ? 0 : overCost(at(a), at(b), k, false);
  };
  /** What two lines cost together: crossing, or fanning into one another from a shared end. */
  const pairCost = (e: number, f: number): number => {
    const [a, b] = edges[e];
    const [c, d] = edges[f];
    const shared = a === c || a === d ? a : b === c || b === d ? b : -1;
    if (shared >= 0) {
      const o = at(shared);
      const u = at(shared === a ? b : a);
      const v = at(shared === c ? d : c);
      return angleBetween({ x: u.x - o.x, y: u.y - o.y }, { x: v.x - o.x, y: v.y - o.y }) < FAN ? COST.fan : 0;
    }
    return crosses(at(a), at(b), at(c), at(d)) ? COST.crossing : 0;
  };
  const areaCost = (): number => {
    let minX = Infinity;
    let maxX = -Infinity;
    let minY = Infinity;
    let maxY = -Infinity;
    for (let i = 0; i < n; i++) {
      const p = at(i);
      minX = Math.min(minX, p.x);
      maxX = Math.max(maxX, p.x);
      minY = Math.min(minY, p.y);
      maxY = Math.max(maxY, p.y);
    }
    return (COST.area * (maxX - minX + 2 * WIDTH) * (maxY - minY + 3)) / (WIDTH * 1.5);
  };
  /**
   * Every term that involves an entity of `moved`: what changes when
   * they move, each term counted once.
   */
  const local = (moved: number[]): number => {
    const lines = new Set(moved.flatMap((i) => incident[i]));
    let cost = areaCost();
    for (const e of lines) cost += lineCost(e);
    for (let f = 0; f < edges.length; f++) {
      if (!lines.has(f)) for (const k of moved) cost += lineOver(f, k);
    }
    for (const e of lines) {
      for (let f = 0; f < edges.length; f++) {
        if (f === e || (lines.has(f) && f < e)) continue;
        cost += pairCost(e, f);
      }
    }
    return cost;
  };

  // Rules 5 and 7: anneal, keeping the best.
  const random = generator(seed);
  const iterations = Math.min(60000, 400 * n);
  const hot = 100;
  const cold = 0.3;
  let current = 0;
  let best = 0;
  let bestPos = pos.slice();
  for (let it = 0; it < iterations; it++) {
    const temperature = hot * Math.pow(cold / hot, it / iterations);
    const i = Math.floor(random() * n);
    const s = Math.floor(random() * slots);
    if (s === pos[i]) continue;
    const j = occupant[s];
    const moved = j >= 0 ? [i, j] : [i];
    const before = local(moved);
    const from = pos[i];
    const apply = (to: number, back: number) => {
      pos[i] = to;
      occupant[to] = i;
      occupant[back] = j;
      if (j >= 0) pos[j] = back;
    };
    // Undone by `apply(from, s)`: a swap keeps the taken slots, a move to an empty slot frees `s` again.
    apply(s, from);
    // Rule 2: a move that splits the island is undone before it is weighed.
    if (j < 0 && !joined()) {
      apply(from, s);
      continue;
    }
    const delta = local(moved) - before;
    if (delta <= 0 || random() < Math.exp(-delta / temperature)) {
      current += delta;
      if (current < best) {
        best = current;
        bestPos = pos.slice();
      }
    } else {
      apply(from, s);
    }
  }

  return new Map(keys.map((k, i) => [k, { q: PITCH * lattice[bestPos[i]].q, r: PITCH * lattice[bestPos[i]].r }]));
}
