// Points and segments: what the placement measures lines with, and
// what the links are cut to their hexes with.

/** A point, in whatever unit its caller works in. */
export type Point = { x: number; y: number };

/** Which side of the line `a`→`b` the point `c` is on: positive, negative or `0` on it. */
const turn = (a: Point, b: Point, c: Point) => (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);

/**
 * Whether the segments `a`–`b` and `c`–`d` cross: each one's ends lie
 * strictly on either side of the other. Segments that only touch, or
 * that lie along one line, do not cross; the placement catches a line
 * along another by its ends, which are entities.
 */
export function crosses(a: Point, b: Point, c: Point, d: Point): boolean {
  const d1 = turn(c, d, a);
  const d2 = turn(c, d, b);
  const d3 = turn(a, b, c);
  const d4 = turn(a, b, d);
  return ((d1 > 0 && d2 < 0) || (d1 < 0 && d2 > 0)) && ((d3 > 0 && d4 < 0) || (d3 < 0 && d4 > 0));
}

/** How far `p` is from the nearest point of the segment `a`–`b`. */
export function distanceToSegment(p: Point, a: Point, b: Point): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const length = dx * dx + dy * dy;
  const t = length === 0 ? 0 : Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / length));
  return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
}

/** The angle between the directions `u` and `v`, in radians, from `0` to `π`. */
export function angleBetween(u: Point, v: Point): number {
  const lengths = Math.hypot(u.x, u.y) * Math.hypot(v.x, v.y);
  if (lengths === 0) return Math.PI;
  return Math.acos(Math.max(-1, Math.min(1, (u.x * v.x + u.y * v.y) / lengths)));
}

/**
 * How far along the direction `d` a point starting at `from`, inside the
 * convex polygon `corners` (about the same origin), travels before it
 * leaves the polygon, in multiples of `d`: the nearest side it meets
 * going forward.
 */
export function exitPolygon(from: Point, d: Point, corners: Point[]): number {
  let best = Infinity;
  for (let k = 0; k < corners.length; k++) {
    const p = corners[k];
    const e = { x: corners[(k + 1) % corners.length].x - p.x, y: corners[(k + 1) % corners.length].y - p.y };
    const denominator = d.x * e.y - d.y * e.x;
    if (denominator === 0) continue;
    const wx = p.x - from.x;
    const wy = p.y - from.y;
    const t = (wx * e.y - wy * e.x) / denominator;
    const s = (wx * d.y - wy * d.x) / denominator;
    if (t > 0 && s >= 0 && s <= 1) best = Math.min(best, t);
  }
  return best === Infinity ? 0 : best;
}
