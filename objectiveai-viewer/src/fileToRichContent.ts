import type { AgentCompletionsMessageRichContentPart } from "@objectiveai/sdk";

/**
 * Convert a browser `File` into an `@objectiveai/sdk`
 * `AgentCompletionsMessageRichContentPart` for inclusion in a user
 * message's content array.
 *
 * MIME-type branching:
 * - `image/*` → `image_url` with a data URI in `url`.
 * - `video/*` → `video_url` with a data URI in `url`.
 * - `audio/*` → `input_audio` with raw base64 in `data` and the
 *   MIME subtype (e.g. `"wav"`, `"mp3"`) in `format`.
 * - everything else → `file` with raw base64 in `file_data` and the
 *   original filename in `filename`.
 *
 * Reads the file eagerly via `FileReader`. Call this at attach time
 * (not send time) so React state stays serializable and the cost is
 * paid once.
 */
export async function fileToRichContentPart(
  file: File,
): Promise<AgentCompletionsMessageRichContentPart> {
  const mime = file.type || "application/octet-stream";

  if (mime.startsWith("image/")) {
    const url = await fileToDataUri(file);
    return { type: "image_url", image_url: { url } };
  }
  if (mime.startsWith("video/")) {
    const url = await fileToDataUri(file);
    return { type: "video_url", video_url: { url } };
  }
  if (mime.startsWith("audio/")) {
    const data = await fileToBase64(file);
    return {
      type: "input_audio",
      input_audio: { data, format: audioFormat(mime, file.name) },
    };
  }

  const file_data = await fileToBase64(file);
  return { type: "file", file: { file_data, filename: file.name } };
}

function fileToDataUri(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onload = () => resolve(r.result as string);
    r.onerror = () => reject(r.error);
    r.readAsDataURL(file);
  });
}

async function fileToBase64(file: File): Promise<string> {
  const dataUri = await fileToDataUri(file);
  return dataUri.replace(/^data:[^;]*;base64,/, "");
}

function audioFormat(mime: string, name: string): string {
  const m = /^audio\/([^;]+)/i.exec(mime);
  if (m && m[1]) return m[1].toLowerCase();
  const dot = name.lastIndexOf(".");
  return dot >= 0 ? name.slice(dot + 1).toLowerCase() : "bin";
}
