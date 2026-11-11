// The hex grid: pointy-topped hexes, as Civilization lays them.

import type { Point } from "./geometry";

/**
 * A hex, in axial coordinates: `q` along a row, `r` the row. Moving one
 * row down moves half a hex right.
 */
export type Hex = { q: number; r: number };

/** A hex in the grid as stored: its column and its row, odd rows half a hex right ("odd-r"). */
export type Offset = { col: number; row: number };

/**
 * The six neighbours' steps, clockwise from east: east, south-east,
 * south-west, west, north-west, north-east. Side `k` of a hex, the one
 * it shares with neighbour `k`, runs from corner `k` to corner `k + 1`.
 */
export const STEPS: readonly Hex[] = [
  { q: 1, r: 0 },
  { q: 0, r: 1 },
  { q: -1, r: 1 },
  { q: -1, r: 0 },
  { q: 0, r: -1 },
  { q: 1, r: -1 },
];

/** How many steps apart two hexes are. */
export function distance(a: Hex, b: Hex): number {
  const dq = a.q - b.q;
  const dr = a.r - b.r;
  return (Math.abs(dq) + Math.abs(dq + dr) + Math.abs(dr)) / 2;
}

/** The six hexes around `h`, in the order of `STEPS`. */
export const neighbours = (h: Hex): Hex[] => STEPS.map((s) => ({ q: h.q + s.q, r: h.r + s.r }));

/** A hex's place in the stored grid. */
export const toOffset = (h: Hex): Offset => ({ col: h.q + (h.r - (h.r & 1)) / 2, row: h.r });

/** The hex at a place in the stored grid. */
export const fromOffset = (o: Offset): Hex => ({ q: o.col - (o.row - (o.row & 1)) / 2, r: o.row });

/** A hex's centre, for hexes whose corners are `1` from their centres. */
export const centre = (h: Hex): Point => ({ x: Math.sqrt(3) * (h.q + h.r / 2), y: 1.5 * h.r });

/** The six corners about a centre, `radius` out, clockwise from the one at the east side's top (−30°). */
export function corners(radius: number): Point[] {
  return [0, 1, 2, 3, 4, 5].map((k) => {
    const angle = ((60 * k - 30) * Math.PI) / 180;
    return { x: radius * Math.cos(angle), y: radius * Math.sin(angle) };
  });
}

/** A hex as a key, for sets and maps. */
export const hexKey = (h: Hex) => `${h.q},${h.r}`;
