// The FUSE serve scope, held: the asks, the file tree, the cancel and
// the end.

import { decodeServerFrame } from "../../../../../../wire/frame/server.js";
import type { Handle, Scope } from "../../../../../../wire/client/index.js";
import { ChannelStream } from "../../../../containers/client/channel_stream.js";
import { OpenError } from "../../../../containers/client/error.js";
import { unary } from "../../../../containers/client/unary.js";
import * as mount from "../../../../../../container_proxy/outside/endpoints/fuse/mount/server/channel_request/frame.js";
import * as ack from "../../../../../../shared/containers/fuse/ack/frame.js";
import * as list from "../../../../../../shared/containers/fuse/list/response/frame.js";
import * as read from "../../../../../../shared/containers/fuse/read/response/frame.js";
import type { Attrs } from "../../../../../../shared/containers/fuse/setattr/attrs.js";
import * as stat from "../../../../../../shared/containers/fuse/stat/response/frame.js";
import type { Error as Refusal } from "../../../../../../shared/error.js";
import type * as tree from "../../../../../../shared/filetree/response/frame.js";
import * as filetree from "../../server/channel_response/filetree.js";
import * as channel_request from "../channel_request/index.js";

/** How a served scope ended. */
export type End = "finished" | "closed";

/** A served volume's file tree: the tree first, then one frame per change, to the scope's end. */
export type FiletreeStream = ChannelStream<tree.Frame, Refusal>;

/**
 * A served volume, held: every ask is one channel and one answer,
 * several in flight at once if wanted; `cancel` is the end the caller
 * makes, and `wait` the end however it comes. The same handle holds a
 * provider's `volumes.fuse_serve` scope and a daemon's, which share
 * their channels byte for byte. Nothing happens on its own: dropping
 * it sends nothing.
 *
 * A serve that ends before its holder cancelled it is a serve lost:
 * every later ask on it fails.
 */
export class ExecuteHandle {
  readonly #handle: Handle;
  readonly #scope: Scope;
  #end: Promise<End> | undefined;

  constructor(handle: Handle, scope: Scope) {
    this.#handle = handle;
    this.#scope = scope;
  }

  /** The scope's number. */
  get scope(): number {
    return this.#scope.scope;
  }

  /**
   * One ask as its bytes already are, and its one answer as the bytes
   * it came as: for a relay, which carries an ask on without reading
   * it. An ask the serve did not answer is the `unanswered` error.
   */
  askRaw(payload: Uint8Array): Promise<Uint8Array> {
    return unary<Uint8Array, never>(this.#handle, this.scope, payload, (answer) => ({ item: answer }));
  }

  /** One ask, and its one answer as the bytes it came as, for the caller to read as that ask's own answer. */
  ask(frame: mount.Frame): Promise<Uint8Array> {
    return this.askRaw(channel_request.encode({ ask: frame }));
  }

  /** What is at `path`. */
  async stat(path: string): Promise<stat.Frame> {
    return stat.decode(await this.ask({ stat: { path } }));
  }

  /** Up to `length` bytes of the file at `path`, from `offset`; fewer is the end of the file. */
  async read(path: string, offset: number, length: number): Promise<read.Frame> {
    return read.decode(await this.ask({ read: { path, offset, length } }));
  }

  /** `bytes` written into the file at `path`, at `offset`. */
  async write(path: string, offset: number, bytes: Uint8Array): Promise<ack.Frame> {
    return ack.decode(await this.ask({ write: { path, offset, bytes } }));
  }

  /** The file at `path` made `size` bytes long. */
  async truncate(path: string, size: number): Promise<ack.Frame> {
    return ack.decode(await this.ask({ truncate: { path, size } }));
  }

  /** The attributes of `path` that `attrs` states, changed. */
  async setattr(path: string, attrs: Attrs): Promise<ack.Frame> {
    return ack.decode(await this.ask({ setattr: { path, attrs } }));
  }

  /** The entries of the directory at `path`. */
  async list(path: string): Promise<list.Frame> {
    return list.decode(await this.ask({ list: { path } }));
  }

  /** The file, or the empty directory, at `path` removed. */
  async remove(path: string): Promise<ack.Frame> {
    return ack.decode(await this.ask({ remove: { path } }));
  }

  /** `from` moved to `to`. */
  async rename(from: string, to: string): Promise<ack.Frame> {
    return ack.decode(await this.ask({ rename: { from, to } }));
  }

  /** A directory made at `path`. */
  async mkdir(path: string): Promise<ack.Frame> {
    return ack.decode(await this.ask({ mkdir: { path } }));
  }

  /** The tree of the volume as this serve sees it, and then its changes, for as long as the scope lives. */
  async filetree(): Promise<FiletreeStream> {
    let channel;
    try {
      channel = await this.#handle.sendChannelRequest(this.scope, channel_request.encode("filetree"));
    } catch (cause) {
      throw new OpenError("send", cause);
    }
    return new ChannelStream<tree.Frame, Refusal>(channel.responseReceiver, (payload) => {
      const answer = filetree.decode(payload);
      return "error" in answer ? { refusal: answer.error } : { item: answer.filetree };
    });
  }

  /** End the serve: the cancel goes out, and this waits for the scope's end. Nothing answers on the cancel's own channel. */
  async cancel(): Promise<void> {
    try {
      await this.#handle.sendChannelRequest(this.scope, channel_request.encode("cancel"));
    } catch (cause) {
      throw new OpenError("send", cause);
    }
    await this.wait();
  }

  /**
   * The scope's end: `finished` at its finish — the cancel answered,
   * or the far end stopped serving on its own — or `closed` where the
   * connection ended first. Resolves once, and answers the same way
   * again after.
   */
  wait(): Promise<End> {
    this.#end ??= this.#read();
    return this.#end;
  }

  async #read(): Promise<End> {
    for (;;) {
      const bytes = await this.#scope.responseReceiver.next();
      if (bytes === undefined) {
        return "closed";
      }
      let envelope;
      try {
        envelope = decodeServerFrame(bytes);
      } catch {
        return "closed";
      }
      if (envelope.kind === "response_finish") {
        return "finished";
      }
    }
  }
}
