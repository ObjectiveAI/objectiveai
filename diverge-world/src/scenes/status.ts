// What the world scene tells the UI scene, through the game's registry.

import type { EdgeType } from "../grid/edges";
import type { Kind } from "../world/model";

/** The registry key of the `Status`. */
export const STATUS = "status";

/** How many of each kind are drawn, and how many lines of each type. */
export type Counts = Record<Kind, number> & Record<EdgeType, number>;

/**
 * Where the world stands: waiting for the daemon — with why the last
 * try failed, if one did — or drawn, with what it holds and the lists
 * the account may not read.
 */
export type Status = { kind: "waiting"; reason: string | null } | { kind: "drawn"; counts: Counts; forbidden: string[] };

/** The registry key of the `Lines`: which types of line the board shows, set by the outliner. */
export const LINES = "lines";

/** Whether each type of line is shown. */
export type Lines = Record<EdgeType, boolean>;

/** The registry key of the `Kinds`: which kinds of node the board shows, set by the outliner. */
export const KINDS = "kinds";

/** Whether each kind of node is shown. */
export type Kinds = Record<Kind, boolean>;

/** The game event the UI sends when the Arrange button is pressed. */
export const ARRANGE = "arrange";

/** The registry key of the `Outline`: every node on the board, as the outliner lists it. */
export const OUTLINE = "outline";

/** Every node on the board: its key, its kind, its name, and whether it is bright. */
export type Outline = { key: string; kind: Kind; label: string; bright: boolean }[];

/** The registry key of the key of the node whose sheet is open, or `null`. */
export const SELECTED = "selected";

/** The game event the outliner sends with a node's key when its row is clicked. */
export const FOCUS = "focus";

/** The game event the outliner sends with a node's key when its row is hovered, and with `null` when left. */
export const HOVER = "hover";

/** The registry key of the outliner's `Bounds`, in the window's pixels. */
export const OUTLINER = "outliner";

/** A rectangle of the window, in its pixels, and whether something on it is being dragged. */
export type Bounds = { x: number; y: number; width: number; height: number; held: boolean };
