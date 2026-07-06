import cn from "classnames";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkBreaks from "remark-breaks";
import remarkGfm from "remark-gfm";

/** Element overrides for rendered agent markdown. Links show as
 * underlined text but never navigate — the webview must not wander
 * off after untrusted agent output. */
const markdownComponents: Components = {
  a: ({ children }) => <span className={cn("underline")}>{children}</span>,
};

/**
 * Untrusted agent markdown, rendered to React elements (no
 * innerHTML) with GFM support and light element styling scoped to
 * the block. Shared by the tree's message boxes and the
 * conversation modal.
 */
export function Markdown({ children }: { children: string }) {
  return (
    <div
      className={cn(
        "[&_p]:mb-1",
        "[&_p:last-child]:mb-0",
        "[&_ul]:list-disc",
        "[&_ul]:pl-4",
        "[&_ol]:list-decimal",
        "[&_ol]:pl-4",
        "[&_blockquote]:border-l",
        "[&_blockquote]:border-copper-mid/50",
        "[&_blockquote]:pl-2",
        "[&_code]:bg-ground-raised",
        "[&_code]:px-0.5",
        "[&_code]:rounded-sm",
        "[&_pre]:overflow-x-auto",
        "[&_pre]:my-1",
        "[&_h1]:font-semibold",
        "[&_h2]:font-semibold",
        "[&_h3]:font-semibold",
        "[&_hr]:border-copper-mid/40",
        "[&_hr]:my-1",
      )}
    >
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkBreaks]}
        components={markdownComponents}
      >
        {children}
      </ReactMarkdown>
    </div>
  );
}
