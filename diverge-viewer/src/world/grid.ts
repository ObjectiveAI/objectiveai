// The world as cells: what every cell is, and where every entity sits.

import type { TileKind } from "./tileset";

/** A cell's place: its column and its row. */
export type Spot = { x: number; y: number };

/** A grid of cells, row by row, and the cell of every entity by its key. */
export type Grid = {
  width: number;
  height: number;
  cells: TileKind[];
  at: Map<string, Spot>;
};

/** The kind of the cell at `x`, `y`. */
export const cell = (grid: Grid, x: number, y: number): TileKind => grid.cells[y * grid.width + x];
