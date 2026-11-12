// The outline's marks, as inline SVG drawn in `currentColor`, so each
// takes its colour from the page's styles: every kind's shape small,
// each type of line's sample, and the few glyphs its buttons carry.

import type { EdgeType } from "../grid/edges";
import type { Kind } from "../world/model";

/** A path, stroked in the text's colour, rounded at its ends and joins. */
const stroke = (d: string, width: number) => `<path d="${d}" stroke="currentColor" stroke-width="${width}" fill="none" stroke-linecap="round" stroke-linejoin="round"/>`;

/** Each kind's shape on a 64 grid, as the board draws it, its lines thick enough to read at 16px. */
const KIND: Record<Kind, string[]> = {
  account: ["M39.72 16.85 A17 17 0 1 1 24.28 16.85", "M25 32 a7 7 0 1 0 14 0 a7 7 0 1 0 -14 0"],
  agent: ["M32 12 L49.32 22 L49.32 42 L32 52 L14.68 42 L14.68 22 Z", "M32 24 L38.93 28 L38.93 36 L32 40 L25.07 36 L25.07 28 Z"],
  standalone_tool: ["M32 13 L51 32 L32 51 L13 32 Z", "M25 32 H39"],
  connected_standalone_tool: ["M32 13 L51 32 L32 51 L13 32 Z", "M24 32 a4.5 4.5 0 1 0 9 0 a4.5 4.5 0 1 0 -9 0", "M31 32 a4.5 4.5 0 1 0 9 0 a4.5 4.5 0 1 0 -9 0"],
  dependency_tool: ["M22.8 22.8 H41.2 V41.2 H22.8 Z"],
  volume: ["M19 23 a13 4.55 0 1 0 26 0 a13 4.55 0 1 0 -26 0", "M19 41 A13 4.55 0 0 0 45 41", "M19 23 V41 M45 23 V41"],
  provider: ["M21 16 H43 V48 H21 Z", "M21 26.5 H43 M21 37.5 H43"],
  daemon: ["M16 32 a16 16 0 1 0 32 0 a16 16 0 1 0 -32 0", "M25.28 32 a6.72 16 0 1 0 13.44 0 a6.72 16 0 1 0 -13.44 0", "M16 32 H48"],
};

/** A kind's shape, 16px. */
export const kindIcon = (kind: Kind) => `<svg class="ico" width="16" height="16" viewBox="0 0 64 64" aria-hidden="true">${KIND[kind].map((d) => stroke(d, 6)).join("")}</svg>`;

/** Each type of line's sample, 24 by 16: a call whole with its chevron, an attachment dashed, a creator dotted from its open dot. */
const LINE: Record<EdgeType, string> = {
  call: `${stroke("M2 8 H21", 1.5)}${stroke("M17 5 L21 8 L17 11", 1.5)}`,
  attachment: `<path d="M2 8 H22" stroke="currentColor" stroke-width="1.5" stroke-dasharray="4 3" fill="none"/>`,
  creator: `${stroke("M2 8 a2 2 0 1 0 4 0 a2 2 0 1 0 -4 0", 1.5)}<path d="M8 8 H22" stroke="currentColor" stroke-width="1.5" stroke-dasharray="1 2.5" stroke-linecap="round" fill="none"/>`,
};

/** A type of line's sample. */
export const lineIcon = (type: EdgeType) => `<svg class="ico" width="24" height="16" viewBox="0 0 24 16" aria-hidden="true">${LINE[type]}</svg>`;

/** The buttons' glyphs, on a 24 grid. */
const GLYPH = {
  arrange: ["M4 4h6v6h-6z", "M14 4h6v6h-6z", "M4 14h6v6h-6z", "M14 14h6v6h-6z"],
  fold: ["M6 15l6 -6l6 6"],
  unfold: ["M6 9l6 6l6 -6"],
  shapes: ["M12 3 L20 8 L20 16 L12 21 L4 16 L4 8 Z"],
  dock: ["M4 4h16v16h-16z", "M9 4v16"],
  float: ["M4 4h16v16h-16z", "M12 13h6v5h-6z"],
  chevron: ["M9 6l6 6l-6 6"],
} as const;

/** A button's glyph, 16px. */
export const glyph = (name: keyof typeof GLYPH) => `<svg class="ico" width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">${GLYPH[name].map((d) => stroke(d, 2)).join("")}</svg>`;
