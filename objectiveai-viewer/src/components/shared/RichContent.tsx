import type { AgentCompletionsMessageRichContentPart } from "objectiveai";
import { hasImageUrl, hasFile } from "../../lib/typeGuards";

export function RichContent({ content }: { content: unknown }) {
  if (typeof content === "string") {
    return <span className="whitespace-pre-wrap break-words leading-relaxed">{content}</span>;
  }
  if (Array.isArray(content)) {
    return (
      <>
        {content.map((part: AgentCompletionsMessageRichContentPart, i: number) => (
          <RichContentPart key={i} part={part} />
        ))}
      </>
    );
  }
  return null;
}

export function RichContentPart({ part }: { part: AgentCompletionsMessageRichContentPart }) {
  if ("text" in part && typeof part.text === "string") {
    return <span className="whitespace-pre-wrap break-words leading-relaxed">{part.text}</span>;
  }
  if (hasImageUrl(part)) {
    return <img className="max-w-[300px] max-h-[200px] rounded-sm mt-1" src={part.image_url.url} alt="image" />;
  }
  if ("input_audio" in part) {
    return <span className="inline-flex items-center gap-1 bg-ground-surface px-2 py-0.5 rounded-sm text-[11px] text-info-dim mt-1">Audio attachment</span>;
  }
  if ("video_url" in part) {
    return <span className="inline-flex items-center gap-1 bg-ground-surface px-2 py-0.5 rounded-sm text-[11px] text-info-dim mt-1">Video attachment</span>;
  }
  if (hasFile(part)) {
    return <span className="inline-flex items-center gap-1 bg-ground-surface px-2 py-0.5 rounded-sm text-[11px] text-info-dim mt-1">{part.file.filename ?? "File"}</span>;
  }
  return null;
}
