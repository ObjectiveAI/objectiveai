// The tileset the world is drawn with: the one you picked, loaded from
// the install folder through Rust (tileset.rs holds it to its rules),
// and every glyph tinted once, as it is first drawn.

import { useEffect, useState } from "react";
import type { Tile } from "../bindings/Tile";
import type { Tiles } from "../bindings/Tiles";
import type { Tileset } from "../bindings/Tileset";
import { app, errorText } from "../lib/app";

/** Every kind of tile the world draws. */
export type TileKind = keyof Tiles;

/** A tileset loaded: its manifest, its atlas with magenta keyed out, and the glyphs tinted so far. */
export type Loaded = {
  id: string;
  tileset: Tileset;
  atlas: HTMLCanvasElement;
  tinted: Map<string, HTMLCanvasElement>;
};

export type TilesetState = { status: "loading" } | { status: "ready"; loaded: Loaded } | { status: "failed"; error: string };

/** The tileset `id`: its manifest and its atlas, both read before anything is drawn. */
async function load(id: string): Promise<Loaded> {
  const [tileset, bytes] = await Promise.all([app.tilesetLoad(id), app.tilesetImage(id)]);
  const bitmap = await createImageBitmap(new Blob([bytes], { type: "image/png" }));
  const atlas = document.createElement("canvas");
  atlas.width = bitmap.width;
  atlas.height = bitmap.height;
  const context = atlas.getContext("2d")!;
  context.drawImage(bitmap, 0, 0);
  bitmap.close();
  // Pure magenta is Dwarf Fortress's transparency key: it shows the background.
  const image = context.getImageData(0, 0, atlas.width, atlas.height);
  const data = image.data;
  for (let i = 0; i < data.length; i += 4) {
    if (data[i] === 255 && data[i + 1] === 0 && data[i + 2] === 255) data[i + 3] = 0;
  }
  context.putImageData(image, 0, 0);
  return { id, tileset, atlas, tinted: new Map() };
}

/**
 * The tile's glyph, multiplied by its foreground: white takes the
 * colour and grey keeps its shading, as in Dwarf Fortress. The glyph's
 * own transparency is kept, so the background shows through it. Made
 * once per glyph and colour.
 */
export function glyph(loaded: Loaded, tile: Tile): HTMLCanvasElement {
  const key = `${tile.glyph}:${tile.fg}`;
  const made = loaded.tinted.get(key);
  if (made) return made;
  const { columns, cell } = loaded.tileset.atlas;
  const sx = (tile.glyph % columns) * cell.width;
  const sy = Math.floor(tile.glyph / columns) * cell.height;
  const canvas = document.createElement("canvas");
  canvas.width = cell.width;
  canvas.height = cell.height;
  const context = canvas.getContext("2d")!;
  context.drawImage(loaded.atlas, sx, sy, cell.width, cell.height, 0, 0, cell.width, cell.height);
  context.globalCompositeOperation = "multiply";
  context.fillStyle = tile.fg;
  context.fillRect(0, 0, cell.width, cell.height);
  context.globalCompositeOperation = "destination-in";
  context.drawImage(loaded.atlas, sx, sy, cell.width, cell.height, 0, 0, cell.width, cell.height);
  loaded.tinted.set(key, canvas);
  return canvas;
}

// Everything showing the tileset, told when a new one is picked.
const listeners = new Set<() => void>();

/** Pick a tileset, or none for the default; whatever shows it loads the one kept. */
export async function pickTileset(id: string | null): Promise<string> {
  const kept = await app.tilesetSet(id);
  for (const listener of listeners) listener();
  return kept;
}

/** The picked tileset, loaded, and loaded again whenever another is picked. */
export function useTileset(): TilesetState {
  const [state, setState] = useState<TilesetState>({ status: "loading" });
  useEffect(() => {
    let live = true;
    const run = () => {
      setState({ status: "loading" });
      app
        .tileset()
        .then(load)
        .then((loaded) => live && setState({ status: "ready", loaded }))
        .catch((error) => live && setState({ status: "failed", error: errorText(error) }));
    };
    run();
    listeners.add(run);
    return () => {
      live = false;
      listeners.delete(run);
    };
  }, []);
  return state;
}
