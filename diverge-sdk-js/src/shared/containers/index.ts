// What the container scopes are made of, as far as the daemon's
// records embed it: the image a container runs, and the dependencies
// an agent's program declares. The rest of the container vocabulary —
// what a program asks its proxy, the secrets contract, FUSE, OCI —
// rides the provider and proxy wires and is not here yet.

export * as request from "./request/index.js";
export * as dependencies from "./dependencies/index.js";
