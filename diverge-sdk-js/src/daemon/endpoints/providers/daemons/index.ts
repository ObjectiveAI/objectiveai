// The daemons reached through the providers: who is announced on
// them, and the direct messages sent through them. `list` lists the
// identities announced on the connected providers, what a
// `daemons` record names a remote by; `listen` hears the messages sent
// to the daemon through the messageable ones; `message` sends one
// through them. Each is judged by the providers_daemons grant, and
// reaches only the providers that grant reaches. Whether the daemon
// announces itself on a provider changes none of the three: it lists a
// provider's announced identities, hears the messages sent to it
// through a messageable provider, and sends a message through a
// provider, announced there or not.

export * as list from "./list/index.js";
export * as listen from "./listen/index.js";
export * as message from "./message/index.js";
