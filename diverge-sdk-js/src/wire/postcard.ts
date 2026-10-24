// Postcard, where the rest of the wire is JSON.
//
// The filetree is the one thing here that is both high-volume and
// free to choose its form, so it rides postcard: no field names, every
// length and integer a varint, an enum its variant's INDEX. This is
// the subset those types need and nothing more — unsigned varints,
// booleans, options, strings and sequences — read and written in the
// order the type declares its members, which is the whole of the
// format's self-description.

const ENCODER = new TextEncoder();
const DECODER = new TextDecoder("utf-8", { fatal: true });

/** Why postcard bytes could not be read. */
export type PostcardErrorKind = "truncated" | "varint" | "bool" | "option" | "utf8" | "trailing";

/** Bytes that are not the type being read. */
export class PostcardError extends Error {
  /** Which. */
  readonly kind: PostcardErrorKind;

  constructor(kind: PostcardErrorKind) {
    super(`postcard: ${kind}`);
    this.name = "PostcardError";
    this.kind = kind;
  }
}

/**
 * A reader over one payload, consuming it front to back. Integers
 * are JavaScript numbers: a varint above `Number.MAX_SAFE_INTEGER`
 * is the `varint` error, since no size or time this wire carries is
 * that large and a `bigint` would infect every type above it.
 */
export class Reader {
  readonly #bytes: Uint8Array;
  #at = 0;

  constructor(bytes: Uint8Array) {
    this.#bytes = bytes;
  }

  /** One byte. */
  #byte(): number {
    const byte = this.#bytes[this.#at];
    if (byte === undefined) {
      throw new PostcardError("truncated");
    }
    this.#at += 1;
    return byte;
  }

  /** An unsigned varint: LEB128, up to ten bytes. */
  varint(): number {
    let value = 0;
    let scale = 1;
    for (let i = 0; i < 10; i += 1) {
      const byte = this.#byte();
      value += (byte & 0x7f) * scale;
      if ((byte & 0x80) === 0) {
        if (!Number.isSafeInteger(value)) {
          throw new PostcardError("varint");
        }
        return value;
      }
      scale *= 128;
    }
    throw new PostcardError("varint");
  }

  /** One byte, `0` or `1`. */
  bool(): boolean {
    const byte = this.#byte();
    if (byte > 1) {
      throw new PostcardError("bool");
    }
    return byte === 1;
  }

  /** A `0` for none, or a `1` and then the value `read`. */
  option<T>(read: (reader: Reader) => T): T | null {
    const byte = this.#byte();
    if (byte === 0) {
      return null;
    }
    if (byte !== 1) {
      throw new PostcardError("option");
    }
    return read(this);
  }

  /** A varint length, then that many bytes of UTF-8. */
  string(): string {
    const length = this.varint();
    const end = this.#at + length;
    if (end > this.#bytes.length) {
      throw new PostcardError("truncated");
    }
    const slice = this.#bytes.subarray(this.#at, end);
    this.#at = end;
    try {
      return DECODER.decode(slice);
    } catch {
      throw new PostcardError("utf8");
    }
  }

  /** A varint count, then that many elements `read`. */
  seq<T>(read: (reader: Reader) => T): T[] {
    const count = this.varint();
    const items: T[] = [];
    for (let i = 0; i < count; i += 1) {
      items.push(read(this));
    }
    return items;
  }

  /** Nothing left: a payload with bytes after its type is `trailing`. */
  end(): void {
    if (this.#at !== this.#bytes.length) {
      throw new PostcardError("trailing");
    }
  }
}

/** A writer building one payload, member by member. */
export class Writer {
  #parts: Uint8Array[] = [];
  #length = 0;

  #push(bytes: Uint8Array): void {
    this.#parts.push(bytes);
    this.#length += bytes.length;
  }

  /** An unsigned varint: LEB128. The value must be a safe integer. */
  varint(value: number): this {
    if (!Number.isSafeInteger(value) || value < 0) {
      throw new RangeError(`not an unsigned safe integer: ${value}`);
    }
    const out: number[] = [];
    let rest = value;
    while (rest >= 128) {
      out.push((rest % 128) | 0x80);
      rest = Math.floor(rest / 128);
    }
    out.push(rest);
    this.#push(Uint8Array.from(out));
    return this;
  }

  /** One byte, `0` or `1`. */
  bool(value: boolean): this {
    this.#push(Uint8Array.of(value ? 1 : 0));
    return this;
  }

  /** A `0` for null, or a `1` and then the value, `write`-n. */
  option<T>(value: T | null, write: (writer: Writer, value: T) => void): this {
    if (value === null) {
      this.#push(Uint8Array.of(0));
    } else {
      this.#push(Uint8Array.of(1));
      write(this, value);
    }
    return this;
  }

  /** A varint length, then the UTF-8. */
  string(value: string): this {
    const bytes = ENCODER.encode(value);
    this.varint(bytes.length);
    this.#push(bytes);
    return this;
  }

  /** A varint count, then every element, `write`-n. */
  seq<T>(items: readonly T[], write: (writer: Writer, item: T) => void): this {
    this.varint(items.length);
    for (const item of items) {
      write(this, item);
    }
    return this;
  }

  /** The payload, as one array. */
  finish(): Uint8Array {
    const out = new Uint8Array(this.#length);
    let at = 0;
    for (const part of this.#parts) {
      out.set(part, at);
      at += part.length;
    }
    return out;
  }
}
