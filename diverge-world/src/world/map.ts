// The whole world as cells: islands placed, landed and packed.

import { cellsUnder, type Point } from "./geometry";
import { islands } from "./islands";
import { land } from "./land";
import type { Entity, Model, Pair } from "./model";
import { pack } from "./pack";
import { PITCH, place } from "./place";
import { cellNoise, hash, valueNoise } from "./random";

/** What a cell is. */
export type Terrain = "ocean" | "shallows" | "land";

/** The world: its size in cells, every cell's terrain and forest, where every entity is, and the pairs. */
export type WorldMap = {
  width: number;
  height: number;
  terrain: Terrain[];
  forest: boolean[];
  at: Map<string, Point>;
  entities: Entity[];
  pairs: Pair[];
  seed: number;
};

/** Cells between an island's box edge and its outermost entity: room for the ring, the coast and the water border. */
const MARGIN = 4;

/**
 * The world a model makes.
 *
 * The rules:
 *
 * 1. Each island is placed (`place`), its slots `PITCH` cells apart and
 *    `MARGIN` cells inside its box, and landed (`land`).
 * 2. The boxes are packed into one sea (`pack`).
 * 3. Water within two cells of land is shallows; the rest is ocean.
 * 4. Forest grows on land at least two cells from every entity and
 *    under no line, where the noise clumps it, so the ring around
 *    every entity stays empty.
 * 5. Everything is seeded by the hash of the entities' keys.
 */
export function build(model: Model): WorldMap {
  const seed = hash(model.entities.map((e) => e.key).sort().join("\n"));

  // Rule 1.
  const placed = islands(model.entities, model.pairs).map((keys) => {
    const members = new Set(keys);
    const pairs = model.pairs.filter((p) => members.has(p.a) && members.has(p.b));
    const islandSeed = hash(keys.join("\n"));
    const slots = place(keys, pairs, islandSeed);
    const local = new Map([...slots].map(([key, s]) => [key, { x: MARGIN + s.col * PITCH, y: MARGIN + s.row * PITCH }]));
    const points = [...local.values()];
    const width = Math.max(...points.map((p) => p.x)) + MARGIN + 1;
    const height = Math.max(...points.map((p) => p.y)) + MARGIN + 1;
    const lines = pairs.map((p): [Point, Point] => [local.get(p.a)!, local.get(p.b)!]);
    return { local, width, height, cells: land(width, height, points, lines, islandSeed) };
  });

  // Rule 2.
  const sea = pack(placed.map(({ width, height }) => ({ width, height })));
  const { width, height } = sea;
  const isLand = new Uint8Array(width * height);
  const at = new Map<string, Point>();
  placed.forEach((island, i) => {
    const o = sea.origins[i];
    for (let y = 0; y < island.height; y++) {
      for (let x = 0; x < island.width; x++) if (island.cells[y * island.width + x] === 1) isLand[(o.y + y) * width + o.x + x] = 1;
    }
    for (const [key, p] of island.local) at.set(key, { x: o.x + p.x, y: o.y + p.y });
  });

  // Rule 3.
  const nearLand = within(isLand, width, height, 2);
  const terrain: Terrain[] = Array.from(isLand, (c, i) => (c === 1 ? "land" : nearLand[i] === 1 ? "shallows" : "ocean"));

  // Rule 4.
  const entityCells = new Uint8Array(width * height);
  for (const p of at.values()) entityCells[p.y * width + p.x] = 1;
  const nearEntity = within(entityCells, width, height, 2);
  const underLine = new Uint8Array(width * height);
  for (const pair of model.pairs) {
    const a = at.get(pair.a);
    const b = at.get(pair.b);
    if (!a || !b) continue;
    for (const c of cellsUnder({ x: a.x + 0.5, y: a.y + 0.5 }, { x: b.x + 0.5, y: b.y + 0.5 }, 0.5)) {
      if (c.x >= 0 && c.y >= 0 && c.x < width && c.y < height) underLine[c.y * width + c.x] = 1;
    }
  }
  const forest = terrain.map((t, i) => {
    if (t !== "land" || nearEntity[i] === 1 || underLine[i] === 1) return false;
    const x = i % width;
    const y = Math.floor(i / width);
    return valueNoise(seed ^ 0x5f3759df, x, y, 4) > 0.55 && cellNoise(seed, x, y) > 0.35;
  });

  return { width, height, terrain, forest, at, entities: model.entities, pairs: model.pairs, seed };
}

/** Which cells lie within `steps` cells — eight ways — of a marked cell, marked ones included. */
function within(marked: Uint8Array, width: number, height: number, steps: number): Uint8Array {
  const out = marked.slice();
  for (let s = 0; s < steps; s++) {
    const before = out.slice();
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        if (before[y * width + x] === 1) continue;
        search: for (let dy = -1; dy <= 1; dy++) {
          for (let dx = -1; dx <= 1; dx++) {
            const nx = x + dx;
            const ny = y + dy;
            if (nx >= 0 && ny >= 0 && nx < width && ny < height && before[ny * width + nx] === 1) {
              out[y * width + x] = 1;
              break search;
            }
          }
        }
      }
    }
  }
  return out;
}
