// One channel, answers to its finish.

import { decodeServerFrame } from "../../../../wire/frame/server.js";
import type { Receiver } from "../../../../wire/client/receiver.js";
import type { Answered } from "./answered.js";
import { ChannelStreamError } from "./error.js";

/**
 * The answers on a channel this side opened, each read by
 * `answered`, until the channel's finish — which ends the iteration
 * and is never an item. A refusal, a frame that could not be read,
 * a frame that does not belong on a channel, or the connection
 * ending is a `ChannelStreamError`, terminal: nothing is read after
 * it. A stream that is not read grows a queue nobody drains.
 */
export class ChannelStream<T, R> implements AsyncIterable<T> {
  #receiver: Receiver<Uint8Array> | undefined;
  readonly #answered: Answered<T, R>;

  constructor(receiver: Receiver<Uint8Array>, answered: Answered<T, R>) {
    this.#receiver = receiver;
    this.#answered = answered;
  }

  /** Whether the channel has ended, by its finish or an error. */
  get terminated(): boolean {
    return this.#receiver === undefined;
  }

  /** The next answer, or `undefined` at the finish. */
  async next(): Promise<T | undefined> {
    if (this.#receiver === undefined) {
      return undefined;
    }
    const bytes = await this.#receiver.next();
    if (bytes === undefined) {
      throw this.#end(new ChannelStreamError<R>("closed"));
    }
    let frame;
    try {
      frame = decodeServerFrame(bytes);
    } catch (cause) {
      throw this.#end(new ChannelStreamError<R>("frame", cause));
    }
    switch (frame.kind) {
      case "channel_response":
        break;
      case "channel_response_finish":
        this.#receiver = undefined;
        return undefined;
      default:
        throw this.#end(new ChannelStreamError<R>("misrouted"));
    }
    let answer;
    try {
      answer = this.#answered(frame.payload);
    } catch (cause) {
      throw this.#end(new ChannelStreamError<R>("response", cause));
    }
    if ("refusal" in answer) {
      throw this.#end(new ChannelStreamError<R>("refused", undefined, answer.refusal));
    }
    return answer.item;
  }

  #end(error: ChannelStreamError<R>): ChannelStreamError<R> {
    this.#receiver = undefined;
    return error;
  }

  async *[Symbol.asyncIterator](): AsyncIterator<T> {
    for (;;) {
      const item = await this.next();
      if (item === undefined) {
        return;
      }
      yield item;
    }
  }
}
