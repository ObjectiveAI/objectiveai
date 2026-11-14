// One direction of a connection, encrypted.

/** The length of a nonce, in bytes. */
const NONCE_LEN = 12;

/** `bytes`, as the buffer WebCrypto's types ask for: every array this package makes is over an ArrayBuffer, never a shared one. */
export function owned(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  return bytes as Uint8Array<ArrayBuffer>;
}

/**
 * One direction of a connection past its handshake: the key that
 * direction's messages are encrypted under. An end holds two — the
 * one it seals what it sends with, and the one it opens what it
 * receives with.
 *
 * 1. The cipher is AES-256-GCM: a message is the ciphertext of one
 *    whole frame, followed by its sixteen-byte tag. No additional
 *    data is authenticated.
 * 2. The nonce is twelve bytes: four zero bytes, then the count of
 *    messages this direction carried before this one, as a `u64`,
 *    big-endian. The first message's count is `0`. The nonce is never
 *    sent: each end counts.
 * 3. A message opens only under the count it was sealed under, so one
 *    dropped, repeated, reordered or altered does not open.
 * 4. A message that does not open ends the connection.
 *
 * The count is the caller's to keep, taken in the order the messages
 * are sent or arrive: sealing and opening are asynchronous here, and
 * the order of the calls, not of their completions, is the order of
 * the messages.
 */
export class Seal {
  readonly #key: CryptoKey;

  private constructor(key: CryptoKey) {
    this.#key = key;
  }

  /** The direction whose key is `key`, thirty-two bytes. */
  static async of(key: Uint8Array): Promise<Seal> {
    return new Seal(await crypto.subtle.importKey("raw", owned(key), "AES-GCM", false, ["encrypt", "decrypt"]));
  }

  /** The nonce of the message whose count is `count` — rule 2. */
  static nonce(count: number): Uint8Array<ArrayBuffer> {
    const nonce = new Uint8Array(NONCE_LEN);
    new DataView(nonce.buffer).setBigUint64(4, BigInt(count), false);
    return nonce;
  }

  /** `frame`, sealed as this direction's message of count `count`. */
  async seal(count: number, frame: Uint8Array): Promise<Uint8Array> {
    return new Uint8Array(await crypto.subtle.encrypt({ name: "AES-GCM", iv: Seal.nonce(count) }, this.#key, owned(frame)));
  }

  /** The frame `message` holds, opened as this direction's message of count `count`; a message that does not open rejects — rules 3 and 4. */
  async open(count: number, message: Uint8Array): Promise<Uint8Array> {
    return new Uint8Array(await crypto.subtle.decrypt({ name: "AES-GCM", iv: Seal.nonce(count) }, this.#key, owned(message)));
  }
}
