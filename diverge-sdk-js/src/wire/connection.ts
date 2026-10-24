// The socket a client holds.
//
// One WebSocket, dialled; what goes out is binary, always, and what
// comes in is read as whole messages. Pings and pongs are the
// platform's business, and a text message, which this protocol has
// none of, is skipped rather than reported. A close and the end of
// the stream are the same thing: the connection is over, so every
// scope on it is over.

import { Receiver } from "./client/receiver.js";

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
 */
export class Connection {
  readonly #socket: WebSocket;
  readonly #frames = new Receiver<Uint8Array>();

  private constructor(socket: WebSocket) {
    this.#socket = socket;
    socket.addEventListener("message", (event) => {
      const data: unknown = event.data;
      if (data instanceof ArrayBuffer) {
        this.#frames.push(new Uint8Array(data));
      }
    });
    socket.addEventListener("close", () => this.#frames.close());
    socket.addEventListener("error", () => this.#frames.close());
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

  /** One binary message out. A socket not open is the `closed` error. */
  send(bytes: Uint8Array): void {
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
