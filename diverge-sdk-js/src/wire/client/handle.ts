// What a caller holds: the four sends.

import type { Channel } from "./channel.js";
import type { Scope } from "./scope.js";

/**
 * The write half of a connection, as every endpoint's `execute`
 * takes it. Four acts: open a scope with a request; open a channel
 * inside one; answer, and finish answering, a channel the SERVER
 * opened. Two things implement it — the socket handle, which speaks
 * the wire itself, and the proxy handle, which asks a Tauri app's
 * Rust to — and nothing above it can tell which it holds. Every
 * payload is written as given, tag and all: this layer does not know
 * what a request says.
 */
export interface Handle {
  /**
   * Open a scope: mint a number, register for what answers it, and
   * write the request. The scope's receivers are live before the
   * frame leaves, so nothing that answers can arrive unregistered.
   */
  sendRequest(payload: Uint8Array): Promise<Scope>;

  /**
   * Open a channel inside `scope`: a number in THIS end's numbering,
   * and the one receiver its answer comes back on. A scope not open
   * is the `scope` send error.
   */
  sendChannelRequest(scope: number, payload: Uint8Array): Promise<Channel>;

  /**
   * Answer a channel the server opened: `scope` and `channel` are
   * quoted from its request, the channel in the SERVER's numbering.
   * Any number of these, including none.
   */
  sendChannelResponse(scope: number, channel: number, payload: Uint8Array): Promise<void>;

  /**
   * End an answer to a channel the server opened. Exactly one, even
   * for an answer that carried nothing: an empty answer and an
   * answer still coming are the same thing until this arrives.
   */
  sendChannelResponseFinish(scope: number, channel: number): Promise<void>;
}
