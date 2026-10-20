// The vault: keys the daemon keeps, each holding one or more JSON
// values, for the secrets a container is handed.
//
// A VAULT KEY is a name and its values, each any JSON, numbered from
// `0` in the order they were appended: its SLOTS. A template declares
// the secrets it needs by name; a container binds each name to a
// vault key; at every activation the daemon hands it one value per
// name — a `fixed` name a random slot's value, read; a `rotating`
// name a free slot's value, LOCKED until the container goes inactive
// and written back. A list reports keys by name and never by value;
// a get alone answers values.

export * as get from "./get/index.js";
export * as insert from "./insert/index.js";
export * as delete from "./delete/index.js";
export * as list from "./list/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
