// The grid, drawn: every cell its tile's background, then its glyph.

import { useEffect, useRef } from "react";
import { type Grid, cell } from "./grid";
import { type Loaded, glyph } from "./tileset";

/** One canvas, the grid at the atlas's own cell size, shown `scale` times larger with its pixels kept square. */
export function Canvas(props: { grid: Grid; loaded: Loaded; scale: number }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const { grid, loaded, scale } = props;
  const { cell: size } = loaded.tileset.atlas;

  useEffect(() => {
    const context = ref.current?.getContext("2d");
    if (!context) return;
    for (let y = 0; y < grid.height; y++) {
      for (let x = 0; x < grid.width; x++) {
        const tile = loaded.tileset.tiles[cell(grid, x, y)];
        context.fillStyle = tile.bg;
        context.fillRect(x * size.width, y * size.height, size.width, size.height);
        context.drawImage(glyph(loaded, tile), x * size.width, y * size.height);
      }
    }
  }, [grid, loaded, size]);

  return (
    <canvas
      ref={ref}
      className="world-canvas"
      width={grid.width * size.width}
      height={grid.height * size.height}
      style={{ width: grid.width * size.width * scale, height: grid.height * size.height * scale }}
    />
  );
}
