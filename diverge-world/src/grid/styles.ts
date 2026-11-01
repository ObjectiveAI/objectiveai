// How each type of line is drawn.

import type { Kind } from "../world/model";
import type { EdgeType } from "./edges";
import { NEON } from "./palette";

/** A line's pattern: whole, in dashes, or in dots. */
export type Pattern = "solid" | "dashed" | "dotted";

/**
 * One type's look: its pattern, its width and alpha, its colour — by
 * the kind of node it runs from, for a type with a way — and what marks
 * its ends: a chevron at the receiver, or an open dot at the maker.
 */
export type Style = { pattern: Pattern; width: number; alpha: number; color: (from: Kind | null) => number; head: "chevron" | "dot" | null };

/** The attachment's neutral: a grey with cyan in it. */
const NEUTRAL = 0x6f8fa0;

/**
 * The three looks:
 *
 * 1. A call: whole, thin and faint, in the sender's neon, a chevron at
 *    the receiver. Its pulses are what shine.
 * 2. An attachment: dashed, neutral, no mark.
 * 3. A creator: dotted, in the maker's neon, an open dot at the maker.
 */
export const STYLES: Record<EdgeType, Style> = {
  call: { pattern: "solid", width: 1.2, alpha: 0.4, color: (from) => (from === null ? NEUTRAL : NEON[from]), head: "chevron" },
  attachment: { pattern: "dashed", width: 1, alpha: 0.45, color: () => NEUTRAL, head: null },
  creator: { pattern: "dotted", width: 1.2, alpha: 0.4, color: (from) => (from === null ? NEUTRAL : NEON[from]), head: "dot" },
};

/** Each type's name on the legend. */
export const NAMES: Record<EdgeType, string> = { call: "calls", attachment: "attachments", creator: "creators" };
