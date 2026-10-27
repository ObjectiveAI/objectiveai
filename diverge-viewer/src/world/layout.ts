// Where everything goes: islands of land in water, an entity on every
// second cell. Provisional — it is laid out again from nothing on every
// change, and keeping places across changes is the next piece.

import type { Grid, Spot } from "./grid";
import type { TileKind } from "./tileset";

/** The kinds of tile that are entities. */
export type EntityKind = Exclude<TileKind, "water" | "land">;

export type Entity = { key: string; kind: EntityKind };

/** Two entities joined, in either direction. */
export type Link = { a: string; b: string };

const ORDER: Record<EntityKind, number> = { account: 0, agent: 1, standalone_tool: 2, dependency_tool: 3 };

/** Water around the whole world, and between any two islands. */
const MARGIN = 2;
const GAP = 2;

/**
 * The grid the entities and their links make.
 *
 * The rules, in order:
 *
 * 1. An island is one connected group of entities, joined by links. An
 *    entity with no link is an island of its own. A link naming an
 *    entity not given joins nothing.
 * 2. Within an island, entities sit on every second cell of a block
 *    about as wide as it is high, ordered by kind (account, agent,
 *    standalone tool, dependency tool) and then by key. No two entities
 *    are next to each other, diagonally included.
 * 3. An island's land is its block with one cell more on every side.
 * 4. Islands are laid out largest first (then by their first key), left
 *    to right in rows about as wide as the whole is high, with two cells
 *    of water between any two and around them all. Every cell no island
 *    covers is water.
 */
export function layout(entities: Entity[], links: Link[]): Grid {
  // Rule 1: the groups, by joining the two ends of every link.
  const parent = new Map(entities.map((e) => [e.key, e.key]));
  const find = (key: string): string => {
    let root = key;
    while (parent.get(root) !== root) root = parent.get(root)!;
    parent.set(key, root);
    return root;
  };
  for (const link of links) {
    if (parent.has(link.a) && parent.has(link.b)) parent.set(find(link.a), find(link.b));
  }
  const groups = new Map<string, Entity[]>();
  for (const e of entities) {
    const root = find(e.key);
    groups.set(root, [...(groups.get(root) ?? []), e]);
  }
  const islands = [...groups.values()].map((members) => {
    members.sort((a, b) => ORDER[a.kind] - ORDER[b.kind] || a.key.localeCompare(b.key));
    // Rule 2 and 3: the block, and the land around it.
    const columns = Math.ceil(Math.sqrt(members.length));
    const rows = Math.ceil(members.length / columns);
    return { members, columns, width: 2 * columns + 1, height: 2 * rows + 1 };
  });
  islands.sort((a, b) => b.members.length - a.members.length || a.members[0].key.localeCompare(b.members[0].key));

  // Rule 4: rows of islands, each row as wide as the target or one island wider.
  const area = islands.reduce((sum, i) => sum + (i.width + GAP) * (i.height + GAP), 0);
  const target = Math.max(...islands.map((i) => i.width), Math.ceil(Math.sqrt(area)));
  const placed: { island: (typeof islands)[number]; origin: Spot }[] = [];
  let x = MARGIN;
  let y = MARGIN;
  let rowHeight = 0;
  let width = 0;
  for (const island of islands) {
    if (x > MARGIN && x + island.width > MARGIN + target) {
      x = MARGIN;
      y += rowHeight + GAP;
      rowHeight = 0;
    }
    placed.push({ island, origin: { x, y } });
    x += island.width + GAP;
    width = Math.max(width, x - GAP + MARGIN);
    rowHeight = Math.max(rowHeight, island.height);
  }
  const height = islands.length === 0 ? 0 : y + rowHeight + MARGIN;

  const cells = new Array<TileKind>(width * height).fill("water");
  const at = new Map<string, Spot>();
  for (const { island, origin } of placed) {
    for (let dy = 0; dy < island.height; dy++) {
      for (let dx = 0; dx < island.width; dx++) cells[(origin.y + dy) * width + origin.x + dx] = "land";
    }
    island.members.forEach((member, i) => {
      const spot = { x: origin.x + 1 + 2 * (i % island.columns), y: origin.y + 1 + 2 * Math.floor(i / island.columns) };
      cells[spot.y * width + spot.x] = member.kind;
      at.set(member.key, spot);
    });
  }
  return { width, height, cells, at };
}
