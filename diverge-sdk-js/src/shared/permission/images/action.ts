// What is done with images.

import { z } from "zod";

/**
 * The actions over images, each the endpoint of the same name under `images`.
 * Snake case on the wire: `"build"`, `"push"`, `"get"`, `"list"`. Reading an
 * image is none of them: `images.read` is held over the providers and
 * the daemons it asks.
 *
 * - `build`: Build an image on a provider, from a Containerfile, into the daemon's Store there, as `images.build` does.
 * - `push`: Push an image into the daemon's Store on a provider, as `images.push` does.
 * - `get`: Get one image in a Store of the daemon's, as `images.get` does.
 * - `list`: List the images in the daemon's Stores, as `images.list` does; the list sends what the grant reaches.
 */
export const Action = z.enum(["build", "push", "get", "list"]);
export type Action = z.infer<typeof Action>;
