// The socket a client holds.
//
// One WebSocket, dialled; what goes out is binary, always, and what
// comes in is read as whole messages. Pings and pongs are the
// platform's business, and a text message, which this protocol has
// none of, is skipped rather than reported. A close and the end of
// the stream are the same thing: the connection is over, so every
// scope on it is over.

import { Receiver } from "./client/receiver.js";
import type { Seal } from "./seal.js";

/** Why a socket could not be opened or written. */
export type ConnectionErrorKind = "open" | "closed" | "send";

/** The socket failing: to open, or to carry one more message. */
export class ConnectionError extends Error {
  /** Which. */
  readonly kind: ConnectionErrorKind;

  constructor(kind: ConnectionErrorKind, cause?: unknown) {
    super(
      kind === "open" ? "the socket could not be opened" : kind === "closed" ? "the socket is closed" : "the socket refused the message",
      cause === undefined ? undefined : { cause },
    );
    this.name = "ConnectionError";
    this.kind = kind;
  }
}

/**
 * A dialled socket: binary frames out through `send`, binary frames
 * in through `frames`, which ends when the socket does. The global
 * `WebSocket` is the one used — Node 22, Deno, Bun and every browser
 * have it — with its binary type set to `arraybuffer` so a message
 * arrives as bytes and never as a Blob.
 *
 * Once `seal` is called — the handshake's last act — every frame
 * crosses the socket encrypted:
 *
 * 1. A frame sent is sealed under this end's direction, and a message
 *    received is opened under the other end's, each under the count
 *    of the messages its direction carried before it.
 * 2. The count is taken as `send` is called and as a message arrives,
 *    and the sealing and the opening, which are asynchronous, run one
 *    after another in that order: frames leave and reach `frames` in
 *    the order they were sent.
 * 3. A message that does not open closes the socket, and `frames`
 *    ends.
 * 4. `frames` ends only after every message that arrived before the
 *    socket's end has been opened.
 */
export class Connection {
  readonly #socket: WebSocket;
  readonly #frames = new Receiver<Uint8Array>();
  /** The two directions, once sealed: this end's, and the other end's. */
  #sealed: { readonly send: Seal; readonly receive: Seal } | undefined;
  /** How many messages each direction has carried since the sealing. */
  #sent = 0;
  #received = 0;
  /** The sealing and the opening still to run, each chain in its order. */
  #outgoing: Promise<void> = Promise.resolve();
  #incoming: Promise<void> = Promise.resolve();

  private constructor(socket: WebSocket) {
    this.#socket = socket;
    socket.addEventListener("message", (event) => {
      const data: unknown = event.data;
      if (!(data instanceof ArrayBuffer)) {
        return;
      }
      const message = new Uint8Array(data);
      const sealed = this.#sealed;
      if (sealed === undefined) {
        this.#frames.push(message);
        return;
      }
      // Rules 1 to 3.
      const count = this.#received++;
      this.#incoming = this.#incoming.then(async () => {
        try {
          this.#frames.push(await sealed.receive.open(count, message));
        } catch {
          this.#frames.close();
          this.#socket.close();
        }
      });
    });
    // Rule 4.
    const ended = () => {
      this.#incoming = this.#incoming.then(() => this.#frames.close());
    };
    socket.addEventListener("close", ended);
    socket.addEventListener("error", ended);
  }

  /** Seal the connection from here on: `send` this end's direction, `receive` the other end's. The handshake's last act, made in the same turn as its last frame is sent. */
  seal(send: Seal, receive: Seal): void {
    this.#sealed = { send, receive };
  }

  /**
   * Dial `url` and wait until the socket is open. A socket that
   * errors or closes before it opens is the `open` error, with what
   * the platform said as its cause where it said anything.
   */
  static open(url: string): Promise<Connection> {
    return new Promise((resolve, reject) => {
      let socket: WebSocket;
      try {
        socket = new WebSocket(url);
      } catch (cause) {
        reject(new ConnectionError("open", cause));
        return;
      }
      socket.binaryType = "arraybuffer";
      const opened = () => {
        socket.removeEventListener("error", failed);
        socket.removeEventListener("close", failed);
        resolve(new Connection(socket));
      };
      const failed = (event: Event) => {
        socket.removeEventListener("open", opened);
        reject(new ConnectionError("open", event));
      };
      socket.addEventListener("open", opened, { once: true });
      socket.addEventListener("error", failed, { once: true });
      socket.addEventListener("close", failed, { once: true });
    });
  }

  /** Every binary message, in order, until the socket ends. */
  get frames(): Receiver<Uint8Array> {
    return this.#frames;
  }

  /**
   * One frame out. A socket not open is the `closed` error. On a
   * sealed connection the frame is sealed first, and leaves after
   * every frame sent before it; a frame that then cannot be sealed or
   * written closes the socket, since the count it took is spent.
   */
  send(bytes: Uint8Array): void {
    if (this.#socket.readyState !== WebSocket.OPEN) {
      throw new ConnectionError("closed");
    }
    const sealed = this.#sealed;
    if (sealed === undefined) {
      this.#write(bytes);
      return;
    }
    const count = this.#sent++;
    this.#outgoing = this.#outgoing.then(async () => {
      try {
        this.#write(await sealed.send.seal(count, bytes));
      } catch {
        this.#socket.close();
      }
    });
  }

  /** One binary message onto the socket, as it is. */
  #write(bytes: Uint8Array): void {
    if (this.#socket.readyState !== WebSocket.OPEN) {
      throw new ConnectionError("closed");
    }
    try {
      // Every array this package makes is over an ArrayBuffer, never
      // a shared one, which is what the socket's type asks.
      this.#socket.send(bytes as Uint8Array<ArrayBuffer>);
    } catch (cause) {
      throw new ConnectionError("send", cause);
    }
  }

  /** Close the socket; every scope on it is over. */
  close(): void {
    this.#socket.close();
  }
}
