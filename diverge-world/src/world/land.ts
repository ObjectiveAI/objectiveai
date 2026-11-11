// An island's land: every hex within `REACH` of an entity, and nothing
// more.

import { distance, type Hex, hexKey } from "./hex";

/** How many hexes out from an entity its land reaches. */
const REACH = 2;

/**
 * The land hexes of one island, each once.
 *
 * The one rule: a hex is land when it is within `REACH` hexes of an
 * entity, the entity's own hex included. No other hex is land.
 */
export function land(entities: Hex[]): Hex[] {
  const cells = new Map<string, Hex>();
  for (const e of entities) {
    for (let dq = -REACH; dq <= REACH; dq++) {
      for (let dr = -REACH; dr <= REACH; dr++) {
        const h = { q: e.q + dq, r: e.r + dr };
        if (distance(h, e) <= REACH) cells.set(hexKey(h), h);
      }
    }
  }
  return [...cells.values()];
}
