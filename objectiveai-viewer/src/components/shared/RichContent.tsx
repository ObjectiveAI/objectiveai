import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { AgentCompletionsMessageRichContentPart } from "@objectiveai/sdk";
import { hasImageUrl, hasFile } from "../../lib/typeGuards";

function MarkdownContent({ text }: { text: string }) {
  const hasMarkdownSyntax = /[#*`\[\]|>~_-]{2}|```/.test(text);
  if (!hasMarkdownSyntax) {
    return <span className="whitespace-pre-wrap break-words leading-relaxed">{text}</span>;
  }
  return (
    <Markdown
      remarkPlugins={[remarkGfm]}
      components={{
        p: ({ children }) => <p className="mb-2 last:mb-0 leading-relaxed">{children}</p>,
        h1: ({ children }) => <h1 className="text-base font-bold text-info-full mt-3 mb-1.5">{children}</h1>,
        h2: ({ children }) => <h2 className="text-sm font-bold text-info-full mt-2.5 mb-1">{children}</h2>,
        h3: ({ children }) => <h3 className="text-sm font-semibold text-info-bright mt-2 mb-1">{children}</h3>,
        code: ({ className, children }) => {
          const isBlock = className?.startsWith("language-");
          if (isBlock) {
            return (
              <pre className="bg-ground border border-node-border rounded-sm px-3 py-2 my-2 overflow-x-auto">
                <code className="text-[11px] font-mono text-info-mid">{children}</code>
              </pre>
            );
          }
          return <code className="bg-ground-surface px-1 py-0.5 rounded-sm text-[11px] font-mono text-copper-mid">{children}</code>;
        },
        pre: ({ children }) => <>{children}</>,
        ul: ({ children }) => <ul className="list-disc list-inside mb-2 space-y-0.5">{children}</ul>,
        ol: ({ children }) => <ol className="list-decimal list-inside mb-2 space-y-0.5">{children}</ol>,
        li: ({ children }) => <li className="text-info-mid">{children}</li>,
        blockquote: ({ children }) => <blockquote className="border-l-2 border-copper-dim pl-3 my-2 text-info-dim italic">{children}</blockquote>,
        a: ({ href, children }) => <a href={href} className="text-copper-bright underline underline-offset-2" target="_blank" rel="noopener noreferrer">{children}</a>,
        table: ({ children }) => <table className="border-collapse border border-node-border my-2 text-xs w-full">{children}</table>,
        th: ({ children }) => <th className="border border-node-border bg-ground-surface px-2 py-1 text-left font-semibold">{children}</th>,
        td: ({ children }) => <td className="border border-node-border px-2 py-1">{children}</td>,
        strong: ({ children }) => <strong className="font-semibold text-info-full">{children}</strong>,
        em: ({ children }) => <em className="italic text-info-mid">{children}</em>,
        hr: () => <hr className="border-node-border my-3" />,
      }}
    >
      {text}
    </Markdown>
  );
}

export function RichContent({ content }: { content: unknown }) {
  if (typeof content === "string") {
    return <MarkdownContent text={content} />;
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
    return <MarkdownContent text={part.text} />;
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
