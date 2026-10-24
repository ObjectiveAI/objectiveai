// The provider protocol: what a caller asks a provider for, and what
// comes back. Only what the daemon's records embed is here so far —
// a volume's mode; the twenty-four endpoints and the caller half are
// not yet mirrored.

export * as endpoints from "./endpoints/index.js";
