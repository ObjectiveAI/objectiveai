// What both ends sign, and draw their keys under.

import { KEY_LEN } from "../frame/auth.js";
import { utf8 } from "../encode.js";
import { Seal, owned } from "../seal.js";

/** What a transcript begins with. */
const LABEL = utf8("diverge handshake");

/** The info the connector's sending key is drawn under. */
const CONNECTOR = utf8("diverge connector");

/** The info the connectee's sending key is drawn under. */
const CONNECTEE = utf8("diverge connectee");

/** Which end signs: the connector's byte is `0`, the connectee's `1`. */
export type Role = "connector" | "connectee";

/**
 * The handshake's transcript: the ASCII `diverge handshake`; the
 * connector's one-time key; the connectee's one-time key; the
 * connector's token behind its length, a `u32`, big-endian; and the
 * connectee's token behind its length, the same way.
 */
export function transcript(connectorKey: Uint8Array, connecteeKey: Uint8Array, connectorToken: string, connecteeToken: string): Uint8Array {
  const tokens = [utf8(connectorToken), utf8(connecteeToken)];
  const out = new Uint8Array(LABEL.length + 2 * KEY_LEN + 8 + (tokens[0] as Uint8Array).length + (tokens[1] as Uint8Array).length);
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

/** Whether `signature` is `role`'s over `transcript`, by the Ed25519 public key `key`. A key that is no key signs nothing. */
export async function proven(transcript: Uint8Array, role: Role, key: Uint8Array, signature: Uint8Array): Promise<boolean> {
  try {
    const verifying = await crypto.subtle.importKey("raw", owned(key), { name: "Ed25519" }, false, ["verify"]);
    return await crypto.subtle.verify({ name: "Ed25519" }, verifying, owned(signature), owned(signed(transcript, role)));
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
