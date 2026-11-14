// What both ends sign, and draw their keys under.

import { POINT_LEN } from "../frame/auth.js";
import { utf8 } from "../encode.js";
import { Seal, owned } from "../seal.js";

/** What a transcript begins with. */
const LABEL = utf8("diverge handshake");

/** The info the connector's sending key is drawn under. */
const CONNECTOR = utf8("diverge connector");

/** The info the connectee's sending key is drawn under. */
const CONNECTEE = utf8("diverge connectee");

/** The length of a session key, in bytes. */
const KEY_LEN = 32;

/** Which end signs: the connector's byte is `0`, the connectee's `1`. */
export type Role = "connector" | "connectee";

/** `bytes` as unpadded base64url. */
function base64url(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

/**
 * The RFC 7638 SHA-256 thumbprint of the P-256 public key `key`, an
 * uncompressed point: the SHA-256 of
 * `{"crv":"P-256","kty":"EC","x":"<x>","y":"<y>"}`, the members in
 * that order with nothing between them, `x` and `y` the 32 bytes of
 * each coordinate as unpadded base64url. It is what a token bound to
 * the key carries as `cnf.jkt`.
 */
export async function thumbprint(key: Uint8Array): Promise<Uint8Array> {
  const canonical = `{"crv":"P-256","kty":"EC","x":"${base64url(key.subarray(1, 33))}","y":"${base64url(key.subarray(33, 65))}"}`;
  return new Uint8Array(await crypto.subtle.digest("SHA-256", owned(utf8(canonical))));
}

/**
 * The handshake's transcript: the ASCII `diverge handshake`; the
 * connector's one-time key; the connectee's one-time key; the
 * connector's token behind its length, a `u32`, big-endian; and the
 * connectee's token behind its length, the same way. The identity
 * keys are not in it: each token names its own, by thumbprint.
 */
export function transcript(connectorKey: Uint8Array, connecteeKey: Uint8Array, connectorToken: string, connecteeToken: string): Uint8Array {
  const tokens = [utf8(connectorToken), utf8(connecteeToken)];
  const out = new Uint8Array(LABEL.length + 2 * POINT_LEN + 8 + (tokens[0] as Uint8Array).length + (tokens[1] as Uint8Array).length);
  const view = new DataView(out.buffer);
  let at = 0;
  for (const part of [LABEL, connectorKey, connecteeKey]) {
    out.set(part, at);
    at += part.length;
  }
  for (const token of tokens) {
    view.setUint32(at, token.length, false);
    out.set(token, at + 4);
    at += 4 + token.length;
  }
  return out;
}

/** What `role` signs: the transcript, then its byte. */
export function signed(transcript: Uint8Array, role: Role): Uint8Array {
  const out = new Uint8Array(transcript.length + 1);
  out.set(transcript, 0);
  out[transcript.length] = role === "connector" ? 0 : 1;
  return out;
}

/**
 * Whether `signature` is `role`'s over `transcript`, by the P-256
 * public key `key`: ECDSA with SHA-256, the signature the 32 bytes of
 * `r` then the 32 bytes of `s`. A key that is no uncompressed point of
 * the curve signs nothing. Either value of `s` a signature may carry
 * is accepted, as the platform's verification accepts both.
 */
export async function proven(transcript: Uint8Array, role: Role, key: Uint8Array, signature: Uint8Array): Promise<boolean> {
  try {
    if (key.length !== POINT_LEN || key[0] !== 4) {
      return false;
    }
    const verifying = await crypto.subtle.importKey("raw", owned(key), { name: "ECDSA", namedCurve: "P-256" }, false, ["verify"]);
    return await crypto.subtle.verify({ name: "ECDSA", hash: "SHA-256" }, verifying, owned(signature), owned(signed(transcript, role)));
  } catch {
    return false;
  }
}

/**
 * The two directions the `shared` secret makes under `transcript`:
 * HKDF-SHA-256, the salt the SHA-256 of the transcript, the input key
 * the shared secret, and thirty-two bytes under each of two infos —
 * `diverge connector`, the key of what the connector sends, and
 * `diverge connectee`, the key of what the connectee sends.
 */
export async function seals(transcript: Uint8Array, shared: Uint8Array): Promise<{ connector: Seal; connectee: Seal }> {
  const salt = new Uint8Array(await crypto.subtle.digest("SHA-256", owned(transcript)));
  const input = await crypto.subtle.importKey("raw", owned(shared), "HKDF", false, ["deriveBits"]);
  const drawn = async (info: Uint8Array) =>
    Seal.of(new Uint8Array(await crypto.subtle.deriveBits({ name: "HKDF", hash: "SHA-256", salt, info: owned(info) }, input, 8 * KEY_LEN)));
  const [connector, connectee] = await Promise.all([drawn(CONNECTOR), drawn(CONNECTEE)]);
  return { connector, connectee };
}
