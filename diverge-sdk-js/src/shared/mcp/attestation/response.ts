// Who answered.

import type { Meta } from "../meta.js";
import { read } from "./read.js";
import { Tool } from "./tool.js";

/** The tool a response's `_meta` is attested with; an `AttestationError` where there is none. */
export function response(meta: Meta): Tool {
  return read(meta, Tool);
}
