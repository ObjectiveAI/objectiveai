// The mode an outgoing provider is dialled in, by name.

import { z } from "zod";

/**
 * Which mode an outgoing provider is dialled in, without the
 * credential: the mode's name alone, what a list and a get report,
 * and what a list narrows by. Snake case on the wire:
 * `"unbrokered"`.
 */
export const Kind = z.enum(["unbrokered"]);
export type Kind = z.infer<typeof Kind>;
