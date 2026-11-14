// Dialling a daemon: the socket, the handshake, the two halves.

import { encodeClientFrame } from "../frame/client.js";
import { Connection } from "../connection.js";
import type { Authenticator, Peer } from "./authenticator.js";
import { authorize } from "./authorize.js";
import { Registry } from "./registry.js";
import { Router } from "./router.js";
import { SocketHandle } from "./socket_handle.js";

/**
 * The URL for `address`: the address itself when it carries a
 * scheme — `wss://daemon.example/` — and otherwise `ws://` in front
 * of it and `/` behind. The daemon accepts the upgrade at any path.
 */
export function url(address: string): string {
  return address.includes("://") ? address : `ws://${address}/`;
}

/**
 * The daemon at `address` dialled, the handshake made as its
 * connector — this end's token presented and its key proven, the
 * daemon's own token verified and the daemon held to the key it
 * names, both through `authenticator` — the connection sealed, so
 * every frame after the handshake crosses the socket encrypted, and
 * split: the router reading on its own, the handle every request is
 * put on, and the daemon, verified. A socket that could
 * not be made is a `ConnectionError`; a handshake that did not
 * complete, a `HandshakeError`, the socket closed.
 */
export async function dial(address: string, authenticator: Authenticator): Promise<{ handle: SocketHandle; peer: Peer }> {
  const connection = await Connection.open(url(address));
  const peer = await authorize(connection, authenticator);
  // A credit lost to a socket that is closing is nothing: the
  // connection is over for every stream it would have paced.
  const registry = new Registry((scope, channel, credit) => {
    try {
      connection.send(encodeClientFrame({ kind: "credit", scope, channel, credit }));
    } catch {
      return;
    }
  });
  const router = new Router(connection, registry);
  void router.run();
  return { handle: new SocketHandle(connection, registry), peer };
}
