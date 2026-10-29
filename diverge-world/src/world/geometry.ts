// Points and segments: what the placement measures lines with, and
// what the links are cut to their cells with.

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
 * How far along the direction `d` a point starting `offset` from the
 * centre of a square box, half `half` wide, travels before it leaves
 * the box, in multiples of `d`. The point starts inside.
 */
export function exitAlong(offset: Point, d: Point, half: number): number {
  let t = Infinity;
  if (d.x > 0) t = Math.min(t, (half - offset.x) / d.x);
  if (d.x < 0) t = Math.min(t, (-half - offset.x) / d.x);
  if (d.y > 0) t = Math.min(t, (half - offset.y) / d.y);
  if (d.y < 0) t = Math.min(t, (-half - offset.y) / d.y);
  return t;
}

/**
 * The cells a segment from `a` to `b` passes over, in cell units,
 * where cell `(x, y)` covers `[x, x + 1)` by `[y, y + 1)`: sampled
 * every tenth of a cell along it, and `spread` either side of it.
 */
export function cellsUnder(a: Point, b: Point, spread: number): Point[] {
  const length = Math.hypot(b.x - a.x, b.y - a.y);
  const nx = length === 0 ? 0 : -(b.y - a.y) / length;
  const ny = length === 0 ? 0 : (b.x - a.x) / length;
  const steps = Math.max(1, Math.ceil(length * 10));
  const seen = new Set<string>();
  const cells: Point[] = [];
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    for (const side of [-spread, 0, spread]) {
      const x = Math.floor(a.x + (b.x - a.x) * t + nx * side);
      const y = Math.floor(a.y + (b.y - a.y) * t + ny * side);
      const key = `${x},${y}`;
      if (seen.has(key)) continue;
      seen.add(key);
      cells.push({ x, y });
    }
  }
  return cells;
}
