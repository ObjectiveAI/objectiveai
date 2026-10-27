// The links, over the grid and apart from it: one straight line per
// joined pair, between the centres of their cells. Each entity's cell
// also holds its name, shown on hover.

import type { Grid } from "./grid";
import type { Link } from "./layout";

export function Links(props: { grid: Grid; links: Link[]; names: Map<string, string>; cell: { width: number; height: number }; scale: number }) {
  const { grid, cell, scale } = props;
  const w = cell.width * scale;
  const h = cell.height * scale;
  // Both directions of a pair are one line.
  const pairs = new Map<string, Link>();
  for (const link of props.links) {
    if (link.a === link.b || !grid.at.has(link.a) || !grid.at.has(link.b)) continue;
    const [a, b] = link.a < link.b ? [link.a, link.b] : [link.b, link.a];
    pairs.set(`${a}\n${b}`, { a, b });
  }
  return (
    <svg className="world-links" width={grid.width * w} height={grid.height * h} aria-hidden>
      {[...pairs.entries()].map(([key, { a, b }]) => {
        const from = grid.at.get(a)!;
        const to = grid.at.get(b)!;
        return <line key={key} className="world-link" x1={(from.x + 0.5) * w} y1={(from.y + 0.5) * h} x2={(to.x + 0.5) * w} y2={(to.y + 0.5) * h} />;
      })}
      {[...grid.at.entries()].map(([key, spot]) => (
        <rect key={key} className="world-spot" x={spot.x * w} y={spot.y * h} width={w} height={h}>
          <title>{props.names.get(key) ?? key}</title>
        </rect>
      ))}
    </svg>
  );
}
