// Who asked.

import type { Meta } from "../meta.js";
import { Caller } from "./caller.js";
import { read } from "./read.js";

/** The caller a request's `_meta` is attested with; an `AttestationError` where there is none. */
export function request(meta: Meta): Caller {
  return read(meta, Caller);
}
