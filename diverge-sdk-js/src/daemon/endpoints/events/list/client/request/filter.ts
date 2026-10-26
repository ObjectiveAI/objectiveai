// What narrows a list of events.

import { z } from "zod";
import { Creator } from "../../../../../../daemon/creator/creator.js";

/**
 * The filter over events: every member optional, and every one given
 * a condition an event passes or does not. A member that lists
 * candidates matches any one of them; an empty list is absent, and
 * matches everything. The graph records no time, so there is no date
 * range.
 */
export const Filter = z.object({
  /** Any one of these on either side, as the sender or as the receiver. */
  parties: z.array(Creator).optional(),
  /** Sent by any one of these. */
  senders: z.array(Creator).optional(),
  /** Received by any one of these. */
  receivers: z.array(Creator).optional(),
});
export type Filter = z.infer<typeof Filter>;
