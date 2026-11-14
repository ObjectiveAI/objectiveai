// The provider's endpoints, as far as the daemon's answers reach
// into them: a volume's mode and a write's content answer, the two
// answers a provider gives about itself that a provider record
// embeds, `info` and `capacity`, the chunks an agent's run streams,
// which the daemon's logs keep, and the read scope's channels, which
// the daemon's own read scope carries.

export * as capacity from "./capacity/index.js";
export * as containers from "./containers/index.js";
export * as images from "./images/index.js";
export * as info from "./info/index.js";
export * as volumes from "./volumes/index.js";
