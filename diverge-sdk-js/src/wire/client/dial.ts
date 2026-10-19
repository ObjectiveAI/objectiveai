// Dialling a daemon: the socket, the credential, the two halves.

import { Connection, ConnectionError } from "../connection.js";
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
 * The daemon at `address` dialled, `key` presented as the one
 * credential frame, and the connection split: the router reading on
 * its own, the handle every request is put on. A socket that could
 * not be made is a `ConnectionError`; the daemon answers the
 * credential with nothing, admitting or closing, so a key it refuses
 * is seen as the first request's scope closing.
 */
export async function dial(address: string, key: string): Promise<SocketHandle> {
  const connection = await Connection.open(url(address));
  try {
    authorize(connection, key);
  } catch (cause) {
    connection.close();
    throw cause instanceof ConnectionError ? cause : new ConnectionError("send", cause);
  }
  const registry = new Registry();
  const router = new Router(connection, registry);
  void router.run();
  return new SocketHandle(connection, registry);
}
