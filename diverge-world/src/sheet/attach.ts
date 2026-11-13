// Files attached to a message: each one read, known by its bytes, and
// made the MCP content block it goes out as.

import type { daemon } from "@diverge/sdk";
import { fileTypeFromBuffer } from "file-type";

/** One content block of a message, MCP's own, as `agents message chat` sends it. */
export type Block = daemon.endpoints.agents.message.chat.client.request.Frame["content"][number];

/** What a file is drawn and sent as: an image, a sound, or any other file. */
export type Kind = "image" | "audio" | "file";

/** A file read and ready to send: its name, its type, what it is, and its block. */
export type Attachment = { name: string; mime: string; kind: Kind; block: Block };

/** How many bytes are turned into characters at once: few enough that no call is handed too many arguments. */
const CHUNK = 0x8000;

/** Bytes as base64, a chunk at a time. */
function base64(bytes: Uint8Array): string {
  let binary = "";
  for (let at = 0; at < bytes.length; at += CHUNK) binary += String.fromCharCode(...bytes.subarray(at, at + CHUNK));
  return btoa(binary);
}

/**
 * A file, read and made its block.
 *
 * The rules:
 *
 * 1. Its type is what its bytes say (`file-type`, by their magic
 *    numbers), so a file named wrongly, or not at all, is still known;
 *    bytes it does not know are the type the file says it is, and with
 *    none `application/octet-stream`.
 * 2. Its block carries its bytes whole, in base64.
 * 3. An `image/` type is an image block and an `audio/` type an audio
 *    block, each its data and its type; anything else is a resource
 *    block, its URI `file:///` and its name, its type, and its bytes as
 *    the resource's `blob`.
 */
export async function attachment(file: File): Promise<Attachment> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  // Rule 1.
  const sniffed = await fileTypeFromBuffer(bytes);
  const mime = sniffed?.mime ?? (file.type || "application/octet-stream");
  // Rule 2.
  const data = base64(bytes);
  // Rule 3.
  const name = file.name;
  if (mime.startsWith("image/")) return { name, mime, kind: "image", block: { type: "image", data, mimeType: mime } };
  if (mime.startsWith("audio/")) return { name, mime, kind: "audio", block: { type: "audio", data, mimeType: mime } };
  return { name, mime, kind: "file", block: { type: "resource", resource: { uri: `file:///${encodeURIComponent(name)}`, mimeType: mime, blob: data } } };
}

/**
 * A message's content: its text, then its files.
 *
 * The rules:
 *
 * 1. Text that is not blank is the first block, as written.
 * 2. The files follow it in the order they were attached.
 */
export function content(text: string, attachments: Attachment[]): Block[] {
  const blocks: Block[] = [];
  // Rule 1.
  if (text.trim() !== "") blocks.push({ type: "text", text });
  // Rule 2.
  for (const attached of attachments) blocks.push(attached.block);
  return blocks;
}
