// What each tool's image calls itself: the `name` of its
// `/DIVERGE.md`, as the daemon's `images info` answers it.

import { daemon, shared } from "@diverge/sdk";
import { handle } from "../lib/wire";

type Image = shared.containers.request.Image;

const info = daemon.endpoints.images.info;

/**
 * The name of every image, by digest.
 *
 * The rules:
 *
 * 1. Each digest is asked once, however many tools run it, and all
 *    are asked at once side by side.
 * 2. An image whose answer is its document is named by the document's
 *    `name`.
 * 3. An image that ships none, that nothing could be asked of, that
 *    the account may not ask about, or whose ask failed has no name
 *    here: the world names its tools some other way. A failure to ask
 *    one image fails no other and not the snapshot.
 */
export async function imageNames(images: Image[]): Promise<Map<string, string>> {
  // Rule 1.
  const unique = new Map<string, Image>();
  for (const image of images) if (!unique.has(image.digest)) unique.set(image.digest, image);
  const asked = await Promise.all(
    [...unique.values()].map(async (image): Promise<[string, string] | null> => {
      try {
        const answer = await info.client.execute(handle, { image });
        // Rule 2; rule 3 for every other answer.
        return typeof answer === "object" && "info" in answer ? [image.digest, answer.info.name] : null;
      } catch {
        // Rule 3.
        return null;
      }
    }),
  );
  return new Map(asked.filter((pair): pair is [string, string] => pair !== null));
}
