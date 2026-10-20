// Taking an attestation out of a `_meta`.

import type { z } from "zod";
import { KEY, type Meta } from "../meta.js";
import { AttestationError } from "./error.js";

/** The value under the key, parsed as `schema`; absent is `missing`, otherwise `invalid`. */
export function read<T extends z.ZodType>(meta: Meta, schema: T): z.infer<T> {
  if (!(KEY in meta)) {
    throw new AttestationError("missing");
  }
  const parsed = schema.safeParse(meta[KEY]);
  if (!parsed.success) {
    throw new AttestationError("invalid", parsed.error);
  }
  return parsed.data;
}
