// What is done to, and through, providers' records.

import { z } from "zod";

/**
 * The actions over providers' records, which a grant reaches as far as its `within` says:
 * the providers whose records the `within` reaches, by their tags.
 * Snake case on the wire: `"get"`, `"list"`, `"read"`, `"edit"`, `"delete"`, `"enable"`, `"disable"`.
 * What is done through them to the daemons connected to them is the `providers_daemons` grant's.
 *
 * - `get`: Read one, as `providers.get` does.
 * - `list`: List them, as `providers.list` does; the list sends what the grant reaches.
 * - `read`: Ask them for images — whether one can be supplied, and its manifests, blobs, proof and DIVERGE.md — as `images.read` does; the daemon asks only the providers the grant reaches.
 * - `edit`: Change one, as `providers.edit` does.
 * - `delete`: Delete one, as `providers.delete` does.
 * - `enable`: Enable one, as `providers.enable` does.
 * - `disable`: Disable one, its runs taken down, as `providers.disable` does.
 */
export const Over = z.enum(["get", "list", "read", "edit", "delete", "enable", "disable"]);
export type Over = z.infer<typeof Over>;
