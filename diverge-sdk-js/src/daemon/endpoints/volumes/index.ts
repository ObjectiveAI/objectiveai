// Volumes: the directories providers hold, reached through the daemon.
//
// A volume is a provider's: a directory the provider offers under a
// name, kept on that provider's disk. The daemon names each by its
// `reference.Volume`, the provider's identity and the name. A volume
// is read, written, walked, resized and deleted at rest; one a
// running container has, or a download, upload or transfer is on, is
// `held`. A delete is refused as `in_use` while any container names
// it in its mounts. A volume carries no creator; its tags are the
// daemon's.

export * as create from "./create/index.js";
export * as get from "./get/index.js";
export * as list from "./list/index.js";
export * as delete from "./delete/index.js";
export * as edit from "./edit/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
export * as stat from "./stat/index.js";
