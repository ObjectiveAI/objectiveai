// The provider's endpoints, as far as the daemon's answers reach
// into them: a volume's mode and a write's content answer, the two
// answers a provider gives about itself that a provider record
// embeds, `info` and `capacity`, and the chunks an agent's run
// streams, which the daemon's logs keep.

export * as capacity from "./capacity/index.js";
export * as containers from "./containers/index.js";
export * as info from "./info/index.js";
export * as volumes from "./volumes/index.js";
