// One dependency, as an image declares it.

import { z } from "zod";
import { bytewise, sha256 } from "../../canonical/index.js";
import { Image, hashed as hashedImage } from "../request/image.js";
import { Service, Transient } from "./mode.js";

const members = {
  /** The image: a digest and its references. */
  image: Image,
  /** The memory the tool container needs, in bytes. */
  memory: z.number().int().nonnegative(),
  /** The bytes the tool container may write, in bytes. */
  disk: z.number().int().nonnegative(),
  /** The arguments the tool container is registered with, verbatim: any JSON, `null` included. */
  arguments: z.json(),
  /** The secrets the image needs, by name, in the order the image states them. Absent, none. */
  secrets: z.array(z.string()).optional(),
};

/**
 * One tool container an agent's program depends on: everything the
 * caller needs to deploy it, as fields and not as words. The image,
 * the limits and the arguments; beside them the secrets the image
 * needs, by name alone; the `mode`, flat with that mode's own
 * members — a transient tool's mounts of the agent's own paths, or a
 * service's scope. What the tool may ask of the caller is not here:
 * its own program declares it at its registration, every time it
 * starts. There is no name: a dependency is named by its
 * id, the SHA-256 of its canonical bytes over `hashed`.
 */
export const Template = z.discriminatedUnion("mode", [Transient.extend(members), Service.extend(members)]);
export type Template = z.infer<typeof Template>;

/**
 * The template as its id hashes it: the same, with the image's
 * references absent, since where the bytes may be fetched is not
 * what the tool is, and the secrets' names sorted bytewise, since
 * the order an image states them in is not what it needs.
 */
export function hashed(template: Template): Template {
  const secrets = template.secrets === undefined ? {} : { secrets: [...template.secrets].sort(bytewise) };
  return { ...template, image: hashedImage(template.image), ...secrets };
}

/** The template's id: sixty-four lowercase hexadecimal characters. */
export function id(template: Template): Promise<string> {
  return sha256(hashed(template));
}
