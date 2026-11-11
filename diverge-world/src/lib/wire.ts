// The page's side of the daemon: the SDK's handle over the app's Rust,
// and the link's two commands.
//
// Every frame crosses Rust through the four proxy commands of the SDK's
// `tauri_proxy` runner; the page never holds the address or the key.
// Every daemon endpoint is
// `daemon.endpoints.<family>.<request>.client.execute(handle, frame)`,
// the frame spread with `relayed(route)`: the daemon it is served by.

import { Channel, invoke } from "@tauri-apps/api/core";
import { type shared, wire } from "@diverge/sdk";

/** How the page reaches the app's Rust: Tauri's `invoke` and its channels, which the SDK's proxy speaks over. */
const transport: wire.proxy.Transport = {
  invoke: (command, args) => invoke(command, args),
  channel: () => new Channel(),
};

/** The one handle: what every endpoint's `execute` is given. */
export const handle: wire.client.Handle = wire.proxy.handle(transport);

/** How the link to the daemon stands, as the SDK states it. */
export type WireStatus = wire.proxy.WireStatus;

/** The link's own commands: how it stands, and dial again now. */
export const link = {
  status: () => wire.proxy.status(transport),
  reset: () => wire.proxy.reset(transport),
};

/**
 * Where a request is served: the IDs of the daemons it is relayed
 * through, our daemon's record of the first, the first's of the
 * second, and on (`shared.relay.Relay`). `[]` is our own daemon.
 */
export type Route = shared.relay.Relay;

/**
 * The member a request carries to be served along `route`.
 *
 * The rules:
 *
 * 1. Our own daemon (`[]`) is nothing: a frame spread with it is the
 *    frame it always was, with no `relay`.
 * 2. Any other route is `relay`, the route whole: our daemon takes the
 *    first ID off and sends the request on with the rest.
 */
export const relayed = (route: Route): { relay?: Route } => (route.length === 0 ? {} : { relay: route });
