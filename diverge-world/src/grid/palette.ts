// The board's colours: all of them the theme's (`lib/theme`), so the
// board is in the same five colours as every panel over it.

import { int } from "../lib/theme";
import type { Kind } from "../world/model";

/**
 * Each kind's family, as a theme role.
 *
 * The rules:
 *
 * 1. Three families, not a colour per kind: who acts (accounts,
 *    agents) in the accent; what they use (the three kinds of tool) in
 *    ink; where it runs (volumes, providers, daemons) in the quietest
 *    words. A node's shape tells kinds apart inside a family.
 * 2. Gold (`here`) is never a kind's: it marks only where you are.
 */
export const FAMILY: Record<Kind, string> = {
  account: "accent",
  agent: "accent",
  standalone_tool: "text",
  connected_standalone_tool: "text",
  dependency_tool: "text",
  volume: "text-3",
  provider: "text-3",
  daemon: "text-3",
};

/** A kind's colour: its family's. */
export const tone = (kind: Kind): number => int(FAMILY[kind]);

/** The field under everything: the theme's ground. */
export const field = (): number => int("bg");

/** The grid's lines, the theme's rule: the fine ones at half strength, every fifth whole. */
export const gridLine = (): number => int("line");

/** A node that is not bright: its shape in the strong edge, its name in the quietest words. */
export const faint = (): number => int("line-strong");
export const faintWords = (): number => int("text-3");

/** A line with no family to take a colour from: an attachment's, or one whose sender is unknown. */
export const neutral = (): number => int("line-strong");

/** What marks working (the trace round a shape), and where you are (the corners round the open node). */
export const ink = (): number => int("text");
export const here = (): number => int("here");

/** A colour as CSS, for the labels. */
export const css = (color: number) => `#${color.toString(16).padStart(6, "0")}`;
