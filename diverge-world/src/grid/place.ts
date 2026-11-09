// Where every node first stands, before anyone drags it, and where one
// that comes later does.

import type { Point } from "../world/geometry";
import type { Entity, Kind } from "../world/model";

/** The columns, left to right. */
const COLUMNS: Kind[] = ["account", "agent", "standalone_tool", "connected_standalone_tool", "dependency_tool", "volume", "provider"];

/** Pixels between columns, between nodes down a column, and round the whole. */
const ACROSS = 240;
const DOWN = 96;
const EDGE = 120;

/**
 * Where a node that came after the first places stands: in its kind's
 * column — the middle of where its kind's nodes stand across — `DOWN`
 * below the lowest of them; or, the first of its kind, a column of its
 * own `ACROSS` right of every node, level with the highest.
 */
export function placeOne(kind: Kind, standing: { kind: Kind; at: Point }[]): Point {
  const mine = standing.filter((s) => s.kind === kind).map((s) => s.at);
  if (mine.length > 0) {
    const xs = mine.map((p) => p.x).sort((a, b) => a - b);
    return { x: xs[Math.floor(xs.length / 2)] ?? EDGE, y: Math.max(...mine.map((p) => p.y)) + DOWN };
  }
  if (standing.length === 0) return { x: EDGE, y: EDGE };
  return { x: Math.max(...standing.map((s) => s.at.x)) + ACROSS, y: Math.min(...standing.map((s) => s.at.y)) };
}

/** Every node's first place, and the size of the board they fill. */
export type Placed = { at: Map<string, Point>; width: number; height: number };

/**
 * The first places.
 *
 * The rules:
 *
 * 1. Each kind is one column, in the order of `COLUMNS`, `ACROSS`
 *    apart; a kind with nothing in it takes no column.
 * 2. Down a column the nodes stand `DOWN` apart, sorted by their
 *    labels, the column centred on the tallest.
 * 3. The links have no say: where a node stands is this, and then
 *    wherever it is dragged.
 */
export function place(entities: Entity[]): Placed {
  const columns = COLUMNS.map((kind) => entities.filter((e) => e.kind === kind).sort((a, b) => a.label.localeCompare(b.label) || a.key.localeCompare(b.key))).filter((c) => c.length > 0);
  const tallest = Math.max(1, ...columns.map((c) => c.length));
  const at = new Map<string, Point>();
  columns.forEach((column, i) => {
    const top = EDGE + ((tallest - column.length) * DOWN) / 2;
    column.forEach((e, j) => at.set(e.key, { x: EDGE + i * ACROSS, y: top + j * DOWN }));
  });
  return { at, width: 2 * EDGE + Math.max(0, columns.length - 1) * ACROSS, height: 2 * EDGE + (tallest - 1) * DOWN };
}
