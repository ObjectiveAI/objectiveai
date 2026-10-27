// The links, over the grid and apart from it: one straight line per
// joined pair, aimed from the centre of one cell to the centre of the
// other and drawn only between their edges, so no line crosses an
// entity's tile. Each entity's cell also holds its name, shown on hover.

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
        const x1 = (from.x + 0.5) * w;
        const y1 = (from.y + 0.5) * h;
        const dx = (to.x + 0.5) * w - x1;
        const dy = (to.y + 0.5) * h - y1;
        // How far along the line a cell's edge is, from its centre: where the line leaves a w × h box.
        const edge = Math.min(dx === 0 ? Infinity : w / 2 / Math.abs(dx), dy === 0 ? Infinity : h / 2 / Math.abs(dy));
        return <line key={key} className="world-link" x1={x1 + dx * edge} y1={y1 + dy * edge} x2={x1 + dx * (1 - edge)} y2={y1 + dy * (1 - edge)} />;
      })}
      {[...grid.at.entries()].map(([key, spot]) => (
        <rect key={key} className="world-spot" x={spot.x * w} y={spot.y * h} width={w} height={h}>
          <title>{props.names.get(key) ?? key}</title>
        </rect>
      ))}
    </svg>
  );
}
