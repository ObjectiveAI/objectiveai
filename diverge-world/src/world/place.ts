// Where each entity of one island goes: the placement that keeps its
// lines out of each other's way.

import { angleBetween, crosses, distanceToSegment, type Point } from "./geometry";
import type { Pair } from "./model";
import { generator } from "./random";

/** A slot of an island's lattice: its column and its row. */
export type Slot = { col: number; row: number };

/** Cells from one slot to the next: two empty cells between any two entities. */
export const PITCH = 3;

/** What each fault costs. */
const COST = {
  /** A line over another entity's cell. */
  through: 500,
  /** A line over an entity's name plate, its own or another's. */
  plate: 150,
  /** Two lines crossing. */
  crossing: 100,
  /** Two lines leaving one entity too close together. */
  fan: 40,
  /** Every cell of every line's length. */
  length: 1,
  /** Every cell of the island's bounding box. */
  area: 0.2,
};

/** A line nearer than this to an entity's centre, in cells, is over its cell. */
const NEAR = 0.6;
/** A line nearer than this to a plate's centre, in cells, is over the plate. */
const PLATE_NEAR = 0.45;
/** Two lines leaving one entity closer than this, in radians, fan into one another. */
const FAN = (20 * Math.PI) / 180;

/**
 * The slots of one island's entities, its top-left slot `(0, 0)`.
 *
 * The rules:
 *
 * 1. Entities sit on a lattice whose slots are `PITCH` cells apart,
 *    so every entity has two empty cells between it and the next, and
 *    its eight neighbours are empty. Its name plate is the cell below
 *    it. The lattice has about twice as many slots as there are
 *    entities, about as wide as it is high, so there is room to move.
 * 2. The first placement is greedy: breadth first from the entity with
 *    the most pairs (then the first key), each entity on the free slot
 *    nearest its placed neighbours.
 * 3. The placement is then improved by simulated annealing: a move
 *    takes one entity to another slot, swapping with the entity there
 *    if there is one, and is kept if it lowers the cost, or by chance
 *    that shrinks as the search cools. The best placement seen is the
 *    answer.
 * 4. The cost is the sum, heaviest first, of: each line over another
 *    entity's cell; each line over a name plate; each two lines that
 *    cross; each two lines leaving one entity less than 20° apart; the
 *    length of every line; and the area of the island. See `COST`.
 * 5. Chance is seeded with `seed`, so the same island always lands the
 *    same way.
 * 6. The answer is moved so its top-left occupied slot is `(0, 0)`.
 */
export function place(keys: string[], pairs: Pair[], seed: number): Map<string, Slot> {
  const n = keys.length;
  if (n === 1) return new Map([[keys[0], { col: 0, row: 0 }]]);

  const index = new Map(keys.map((k, i) => [k, i]));
  const edges: [number, number][] = pairs.filter((p) => index.has(p.a) && index.has(p.b)).map((p) => [index.get(p.a)!, index.get(p.b)!]);
  const incident: number[][] = keys.map(() => []);
  edges.forEach(([a, b], e) => {
    incident[a].push(e);
    incident[b].push(e);
  });

  // Rule 1: the lattice.
  const cols = Math.ceil(Math.sqrt(2 * n));
  const rows = Math.ceil((2 * n) / cols);
  const slots = cols * rows;
  const pos = new Int32Array(n).fill(-1);
  const occupant = new Int32Array(slots).fill(-1);
  const at = (i: number): Point => ({ x: (pos[i] % cols) * PITCH, y: Math.floor(pos[i] / cols) * PITCH });
  const slotPoint = (s: number): Point => ({ x: (s % cols) * PITCH, y: Math.floor(s / cols) * PITCH });

  // Rule 2: breadth first from the most joined, each beside its placed neighbours.
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
  const middle = Math.floor(rows / 2) * cols + Math.floor(cols / 2);
  for (const i of order) {
    const placed = neighbours[i].filter((j) => pos[j] >= 0).map(at);
    let best = -1;
    let bestScore = Infinity;
    for (let s = 0; s < slots; s++) {
      if (occupant[s] >= 0) continue;
      const p = slotPoint(s);
      const centre = slotPoint(middle);
      const score = placed.reduce((sum, q) => sum + Math.hypot(p.x - q.x, p.y - q.y), 0) + 0.01 * Math.hypot(p.x - centre.x, p.y - centre.y);
      if (score < bestScore) {
        bestScore = score;
        best = s;
      }
    }
    pos[i] = best;
    occupant[best] = i;
  }

  // Rule 4: the cost, a term at a time.
  const plateOf = (p: Point): Point => ({ x: p.x, y: p.y + 1 });
  /** What a line costs on its own: its length, and what it passes over. */
  const lineCost = (e: number): number => {
    const [a, b] = edges[e];
    const pa = at(a);
    const pb = at(b);
    let cost = COST.length * Math.hypot(pb.x - pa.x, pb.y - pa.y);
    for (let k = 0; k < n; k++) {
      const pk = at(k);
      if (k !== a && k !== b && distanceToSegment(pk, pa, pb) < NEAR) cost += COST.through;
      if (distanceToSegment(plateOf(pk), pa, pb) < PLATE_NEAR) cost += COST.plate;
    }
    return cost;
  };
  /** What a line costs for passing over entity `k`, which is not one of its ends. */
  const overCost = (e: number, k: number): number => {
    const [a, b] = edges[e];
    if (k === a || k === b) return 0;
    const pa = at(a);
    const pb = at(b);
    const pk = at(k);
    let cost = 0;
    if (distanceToSegment(pk, pa, pb) < NEAR) cost += COST.through;
    if (distanceToSegment(plateOf(pk), pa, pb) < PLATE_NEAR) cost += COST.plate;
    return cost;
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
    let minC = Infinity;
    let maxC = -Infinity;
    let minR = Infinity;
    let maxR = -Infinity;
    for (let i = 0; i < n; i++) {
      const c = pos[i] % cols;
      const r = Math.floor(pos[i] / cols);
      minC = Math.min(minC, c);
      maxC = Math.max(maxC, c);
      minR = Math.min(minR, r);
      maxR = Math.max(maxR, r);
    }
    return COST.area * ((maxC - minC) * PITCH + 1) * ((maxR - minR) * PITCH + 1);
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
      if (!lines.has(f)) for (const k of moved) cost += overCost(f, k);
    }
    for (const e of lines) {
      for (let f = 0; f < edges.length; f++) {
        if (f === e || (lines.has(f) && f < e)) continue;
        cost += pairCost(e, f);
      }
    }
    return cost;
  };

  // Rule 3 and 5: anneal, keeping the best.
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
    pos[i] = s;
    occupant[s] = i;
    occupant[from] = j;
    if (j >= 0) pos[j] = from;
    const delta = local(moved) - before;
    if (delta <= 0 || random() < Math.exp(-delta / temperature)) {
      current += delta;
      if (current < best) {
        best = current;
        bestPos = pos.slice();
      }
    } else {
      pos[i] = from;
      occupant[from] = i;
      occupant[s] = j;
      if (j >= 0) pos[j] = s;
    }
  }

  // Rule 6.
  const minCol = Math.min(...Array.from(bestPos, (s) => s % cols));
  const minRow = Math.min(...Array.from(bestPos, (s) => Math.floor(s / cols)));
  return new Map(keys.map((k, i) => [k, { col: (bestPos[i] % cols) - minCol, row: Math.floor(bestPos[i] / cols) - minRow }]));
}
