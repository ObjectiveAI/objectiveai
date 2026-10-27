// What is done with images.

import { z } from "zod";

/**
 * The actions over images, each the endpoint of the same name under `images`.
 * Snake case on the wire: `"info"`.
 *
 * - `info`: Read an image's DIVERGE.md.
 */
export const Action = z.enum(["info"]);
export type Action = z.infer<typeof Action>;
