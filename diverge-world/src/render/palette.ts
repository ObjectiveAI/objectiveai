// The world's colours: Civilization's sea, grass and sand, and one
// colour for each kind of entity.

import type { Kind } from "../world/model";

export const SEA = 0x1f4a7a;
export const GRASS = 0x5e9a3a;
export const SAND = 0xdcc68c;
export const SHORE = 0x24361c;

/** The ink lines and outlines are drawn with. */
export const INK = 0x16120a;
/** The cream the links are drawn in. */
export const CREAM = 0xf5ead0;

/** Each kind's shield. */
export const KIND: Record<Kind, number> = {
  account: 0xd9a521,
  agent: 0x2f6fd0,
  standalone_tool: 0xc0392b,
  dependency_tool: 0x8044b8,
};

/** `color` made lighter by `amount` (towards white) or darker (negative, towards black). */
export function shade(color: number, amount: number): number {
  const channel = (shift: number) => {
    const c = (color >> shift) & 0xff;
    const v = amount >= 0 ? c + (255 - c) * amount : c * (1 + amount);
    return Math.max(0, Math.min(255, Math.round(v))) << shift;
  };
  return channel(16) | channel(8) | channel(0);
}
