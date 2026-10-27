// What the container scopes are made of, as far as the daemon's
// exchanges reach it: the image a container runs, the dependencies
// an agent's program declares, and a piece of a file being written
// in. The rest of the container vocabulary — what a program asks its
// proxy, the secrets contract, FUSE, OCI — rides the provider and
// proxy wires and is not here yet.

export * as request from "./request/index.js";
export * as dependencies from "./dependencies/index.js";
export * as info from "./info/index.js";
export * as write_bytes from "./write_bytes/index.js";
