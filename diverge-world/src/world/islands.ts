// Which entities share an island.

import type { Entity, Pair } from "./model";

/**
 * The islands: one per connected group of entities.
 *
 * The rules:
 *
 * 1. Two entities joined by a pair are on one island, and so is every
 *    entity joined to either, and so on.
 * 2. An entity joined to nothing is an island of its own.
 * 3. A pair naming an entity not given joins nothing.
 * 4. Each island's keys are sorted, and the islands are sorted by
 *    their first key, so the same entities always make the same islands.
 */
export function islands(entities: Entity[], pairs: Pair[]): string[][] {
  const parent = new Map(entities.map((e) => [e.key, e.key]));
  const find = (key: string): string => {
    let root = key;
    while (parent.get(root) !== root) root = parent.get(root)!;
    parent.set(key, root);
    return root;
  };
  for (const pair of pairs) {
    if (parent.has(pair.a) && parent.has(pair.b)) parent.set(find(pair.a), find(pair.b));
  }
  const groups = new Map<string, string[]>();
  for (const e of entities) {
    const root = find(e.key);
    groups.set(root, [...(groups.get(root) ?? []), e.key]);
  }
  const result = [...groups.values()].map((keys) => keys.sort());
  return result.sort((a, b) => (a[0] < b[0] ? -1 : 1));
}
