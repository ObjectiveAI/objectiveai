// The container proxy's endpoints, as far as the daemon's wire reaches
// into them: the asks a FUSE mount makes, which a volume's serve
// answers.

export * as fuse from "./fuse/index.js";
