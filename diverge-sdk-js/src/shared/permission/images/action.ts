// What is done with images.

import { z } from "zod";

/**
 * The actions over images, each the endpoint of the same name under `images`.
 * Snake case on the wire: `"build"`. Reading an image is none of
 * them: `images.read` is held over the providers and the daemons it
 * asks.
 *
 * - `build`: Build an image on a provider, from a Containerfile.
 */
export const Action = z.enum(["build"]);
export type Action = z.infer<typeof Action>;
