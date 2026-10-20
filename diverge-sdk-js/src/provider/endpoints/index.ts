// The provider's endpoints, as far as the daemon's records reach
// into them: a volume's mode, and the two answers a provider gives
// about itself that a provider record embeds, `info` and `capacity`.

export * as capacity from "./capacity/index.js";
export * as info from "./info/index.js";
export * as volumes from "./volumes/index.js";
