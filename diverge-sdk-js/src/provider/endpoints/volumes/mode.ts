// What becomes of a change to a volume.

import { z } from "zod";

/**
 * What becomes of a change to a volume — kept, discarded, or never
 * made — and so who may hold the volume at once. A fact of the
 * volume: stated by a create, reported by a listing, changed only by
 * an edit, and never a mount's to change; each mount STATES the mode
 * it means the volume to have, and a volume in another mode refuses
 * it. `persistent`: every change is kept; `ephemeral`: every change
 * is taken and discarded afterwards, with the container or with the
 * FUSE serve; `read_only`: no change is made. On the provider's wire
 * one postcard varint, `0`, `1`, `2`; as JSON the three strings.
 */
export const Mode = z.enum(["persistent", "ephemeral", "read_only"]);
export type Mode = z.infer<typeof Mode>;
