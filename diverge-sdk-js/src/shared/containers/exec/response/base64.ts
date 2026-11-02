// A command's bytes as JSON: standard base64.

const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/** Bytes as standard base64, padded: how the JSON form of a stdout or stderr frame carries them. */
export function base64(bytes: Uint8Array): string {
  let out = "";
  for (let at = 0; at < bytes.length; at += 3) {
    const a = bytes[at] ?? 0;
    const b = bytes[at + 1] ?? 0;
    const c = bytes[at + 2] ?? 0;
    const triple = (a << 16) | (b << 8) | c;
    out += ALPHABET[(triple >> 18) & 63];
    out += ALPHABET[(triple >> 12) & 63];
    out += at + 1 < bytes.length ? ALPHABET[(triple >> 6) & 63] : "=";
    out += at + 2 < bytes.length ? ALPHABET[triple & 63] : "=";
  }
  return out;
}
