// How each type of line is drawn.

import type { Kind } from "../world/model";
import type { EdgeType } from "./edges";
import { neutral, tone } from "./palette";

/** A line's pattern: whole, in dashes, or in dots. */
export type Pattern = "solid" | "dashed" | "dotted";

/**
 * One type's look: its pattern, its width and alpha, its colour — by
 * the kind of node it runs from, for a type with a way — and what marks
 * its ends: a chevron at the receiver, or an open dot at the maker.
 */
export type Style = { pattern: Pattern; width: number; alpha: number; color: (from: Kind | null) => number; head: "chevron" | "dot" | null };

/**
 * The three looks, flat — no glow:
 *
 * 1. A call: whole, in the sender's family, a chevron at the receiver.
 *    What passes along it runs as a short stroke (`pulses`).
 * 2. An attachment: dashed, in the strong edge, no mark.
 * 3. A creator: dotted, in the maker's family, an open dot at the maker.
 */
export const STYLES: Record<EdgeType, Style> = {
  call: { pattern: "solid", width: 1.5, alpha: 1, color: (from) => (from === null ? neutral() : tone(from)), head: "chevron" },
  attachment: { pattern: "dashed", width: 1.5, alpha: 1, color: () => neutral(), head: null },
  creator: { pattern: "dotted", width: 1.5, alpha: 1, color: (from) => (from === null ? neutral() : tone(from)), head: "dot" },
};
