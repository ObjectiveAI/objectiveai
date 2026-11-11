// The whole world as hexes: islands placed, landed and packed.

import { type Hex, toOffset } from "./hex";
import { islands } from "./islands";
import { land } from "./land";
import type { Entity, Model, Pair } from "./model";
import { pack } from "./pack";
import { place } from "./place";
import { hash } from "./random";

/**
 * The world: its size in columns and rows of the stored grid, which
 * hexes are land, where every entity is, and the pairs.
 */
export type WorldMap = {
  columns: number;
  rows: number;
  /** `1` for land, `0` for sea, row by row of the stored grid. */
  land: Uint8Array;
  at: Map<string, Hex>;
  entities: Entity[];
  pairs: Pair[];
};

/** Whether the hex `h` is land: `false` off the map. */
export function isLand(map: WorldMap, h: Hex): boolean {
  const { col, row } = toOffset(h);
  return col >= 0 && row >= 0 && col < map.columns && row < map.rows && map.land[row * map.columns + col] === 1;
}

/**
 * The world a model makes.
 *
 * The rules:
 *
 * 1. Each island is placed (`place`), seeded by the hash of its keys,
 *    and landed (`land`): its entities' hexes and their rings, nothing
 *    more.
 * 2. The islands are packed into one sea (`pack`).
 * 3. Every other hex is sea.
 */
export function build(model: Model): WorldMap {
  // Rule 1.
  const placed = islands(model.entities, model.pairs).map((keys) => {
    const members = new Set(keys);
    const pairs = model.pairs.filter((p) => members.has(p.a) && members.has(p.b));
    const at = place(keys, pairs, hash(keys.join("\n")));
    return { at, land: land([...at.values()]) };
  });

  // Rules 2 and 3.
  const sea = pack(placed.map((island) => island.land));
  const { columns, rows } = sea;
  const cells = new Uint8Array(columns * rows);
  const at = new Map<string, Hex>();
  placed.forEach((island, i) => {
    const m = sea.moves[i];
    for (const h of island.land) {
      const { col, row } = toOffset({ q: h.q + m.q, r: h.r + m.r });
      cells[row * columns + col] = 1;
    }
    for (const [key, h] of island.at) at.set(key, { q: h.q + m.q, r: h.r + m.r });
  });
  return { columns, rows, land: cells, at, entities: model.entities, pairs: model.pairs };
}
