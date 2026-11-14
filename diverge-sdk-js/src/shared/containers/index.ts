// What the container scopes are made of, as far as the daemon's
// exchanges reach it: the image a container runs, the dependencies
// an agent's program declares, and a piece of a file being written
// in, the OCI asks over one image — a manifest, a blob, the
// proof — that the daemon's read scope carries, and the FUSE
// vocabulary the daemon's serve of a volume carries. The rest of the
// container vocabulary — what a program asks its proxy, the secrets
// contract — rides the provider and proxy wires and is not here yet.

export * as request from "./request/index.js";
export * as response from "./response/index.js";
export * as fuse from "./fuse/index.js";
export * as dependencies from "./dependencies/index.js";
export * as info from "./info/index.js";
export * as write_bytes from "./write_bytes/index.js";
export * as exec from "./exec/index.js";
export * as terminal from "./terminal/index.js";
export * as frontend from "./frontend/index.js";
export * as oci from "./oci/index.js";
export * as structured_decisions from "./structured_decisions/index.js";
