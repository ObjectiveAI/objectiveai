// What each tool's image calls itself — the `name` of its
// `/DIVERGE.md`, as the daemon's `images read` answers it — and whether
// it serves a frontend.

import { daemon, shared } from "@diverge/sdk";
import { handle, type Route, relayed } from "../lib/wire";

type Image = shared.containers.request.Image;

const read = daemon.endpoints.images.read;

/**
 * The name of every image, by digest.
 *
 * The rules:
 *
 * 1. Each digest is asked once, however many tools run it, and all
 *    are asked at once side by side.
 * 2. An image whose answer is its document is named by the document's
 *    `name`.
 * 3. An image that ships none, that no source the account reaches
 *    holds, that the account may not ask about, or whose ask failed
 *    has no name
 *    here: the world names its tools some other way. A failure to ask
 *    one image fails no other and not the snapshot.
 * 4. Every image is asked of the daemon `route` reaches (`relayed`): the
 *    one whose tools run it.
 * 5. Each ask is one read scope, held for its one question — the
 *    image's document — and cancelled after it, answered or not.
 */
export async function imageNames(images: Image[], route: Route): Promise<Map<string, string>> {
  const found = await imageInfo(images, route);
  return new Map([...found].map(([digest, info]) => [digest, info.name]));
}

/** An image as the world names it: its document's `name`, and whether its program serves a frontend. */
export type ImageInfo = { name: string; frontend: boolean };

/** Every image's name and frontend, by digest, by the rules of `imageNames`. */
export async function imageInfo(images: Image[], route: Route): Promise<Map<string, ImageInfo>> {
  // Rule 1.
  const unique = new Map<string, Image>();
  for (const image of images) if (!unique.has(image.digest)) unique.set(image.digest, image);
  const asked = await Promise.all(
    [...unique.values()].map(async (image): Promise<[string, ImageInfo] | null> => {
      try {
        // Rule 4.
        const held = await read.client.execute(handle, { image, ...relayed(route) });
        // Rule 5.
        try {
          const info = await held.info();
          // Rule 2; rule 3 for an image that ships none.
          return info === undefined ? null : [image.digest, { name: info.name, frontend: info.frontend === true }];
        } finally {
          await held.cancel().catch(() => undefined);
        }
      } catch {
        // Rule 3.
        return null;
      }
    }),
  );
  return new Map(asked.filter((pair): pair is [string, ImageInfo] => pair !== null));
}
