// The board's colours: a dark field, a faint grid, one neon per kind.

import type { Kind } from "../world/model";

/** The field under everything. */
export const FIELD = 0x05070d;

/** The grid's lines: the fine ones, and every fifth. */
export const GRID_FINE = 0x0d2a36;
export const GRID_BOLD = 0x14485c;

/** Each kind's neon. */
export const NEON: Record<Kind, number> = {
  account: 0xffe14d,
  agent: 0x22e5ff,
  standalone_tool: 0xff8a1f,
  connected_standalone_tool: 0x9b7bff,
  dependency_tool: 0xff3df0,
  volume: 0x39ff88,
};

/** A colour as CSS, for the labels. */
export const css = (color: number) => `#${color.toString(16).padStart(6, "0")}`;
