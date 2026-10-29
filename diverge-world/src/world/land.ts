// An island's land: each entity's hex and the six around it, and
// nothing more.

import { type Hex, hexKey, neighbours } from "./hex";

/**
 * The land hexes of one island, each once.
 *
 * The one rule: a hex is land when it is an entity's hex or one of an
 * entity's six neighbours. No other hex is land.
 */
export function land(entities: Hex[]): Hex[] {
  const cells = new Map<string, Hex>();
  for (const e of entities) {
    for (const h of [e, ...neighbours(e)]) cells.set(hexKey(h), h);
  }
  return [...cells.values()];
}
