// What is done with images.

import { z } from "zod";

/**
 * The actions over images, each the endpoint of the same name under `images`.
 * Snake case on the wire: `"info"`, `"build"`.
 *
 * - `info`: Read an image's DIVERGE.md.
 * - `build`: Build an image on a provider, from a Containerfile.
 */
export const Action = z.enum(["info", "build"]);
export type Action = z.infer<typeof Action>;
