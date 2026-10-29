// What the world scene tells the UI scene, through the game's registry.

import type { Entity, Kind } from "../world/model";

/** The registry key of the `Status`. */
export const STATUS = "status";
/** The registry key of the entity under the pointer, or `null`. */
export const HOVER = "hover";

/** How many of each kind are drawn, and how many links. */
export type Counts = Record<Kind, number> & { links: number };

/**
 * Where the world stands: waiting for the daemon — with why the last
 * try failed, if one did — or drawn, with what it holds and the lists
 * the account may not read.
 */
export type Status = { kind: "waiting"; reason: string | null } | { kind: "drawn"; counts: Counts; forbidden: string[] };

/** What the hover panel shows. */
export type Hover = Entity | null;
