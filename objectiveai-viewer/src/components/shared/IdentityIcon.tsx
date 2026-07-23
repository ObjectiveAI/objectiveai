import { useEffect, useState } from "react";
import cn from "classnames";

/**
 * An identity's tab icon, with BOTH color behaviors: an SVG is
 * fetched and INLINED into this document, so explicit fills render
 * as authored while `currentColor` fills inherit the surrounding
 * text color — the icon chooses. Anything that isn't inlinable SVG
 * (a PNG, a fetch failure) falls back to a plain `<img>` (own
 * pixels, hidden on error).
 *
 * Inlined markup is sanitized (scripts + on* handlers stripped) —
 * plugin-supplied icons will flow through here. Fetches are cached
 * per URL for the document's life (every tab re-renders the strip).
 */

const cache = new Map<string, Promise<string | null>>();

/** Strip executable surface from untrusted SVG markup. */
function sanitizeSvg(svg: string): string {
  return svg
    .replace(/<script[\s\S]*?<\/script\s*>/gi, "")
    .replace(/\son[a-z]+\s*=\s*"[^"]*"/gi, "")
    .replace(/\son[a-z]+\s*=\s*'[^']*'/gi, "");
}

/** The URL's sanitized SVG text, or `null` when it isn't SVG (or
 * isn't fetchable) — the `<img>` fallback path. */
function fetchSvg(url: string): Promise<string | null> {
  let pending = cache.get(url);
  if (!pending) {
    pending = (async () => {
      try {
        const response = await fetch(url);
        if (!response.ok) return null;
        const type = response.headers.get("content-type") ?? "";
        const text = await response.text();
        if (!type.includes("svg") && !text.trimStart().startsWith("<svg")) {
          return null;
        }
        return sanitizeSvg(text);
      } catch {
        return null;
      }
    })();
    cache.set(url, pending);
  }
  return pending;
}

export function IdentityIcon({
  url,
  className,
}: {
  url: string;
  /** Size + layout classes (e.g. `w-2.5 h-2.5 shrink-0`). */
  className?: string;
}) {
  // undefined = still fetching (render nothing yet — no flash of the
  // fallback), string = inline SVG, null = <img> fallback.
  const [svg, setSvg] = useState<string | null | undefined>(undefined);
  useEffect(() => {
    let disposed = false;
    setSvg(undefined);
    void fetchSvg(url).then((text) => {
      if (!disposed) setSvg(text);
    });
    return () => {
      disposed = true;
    };
  }, [url]);

  if (svg === undefined) return null;
  if (svg === null) {
    return (
      <img
        data-identity-icon
        src={url}
        alt=""
        draggable={false}
        onError={(e) => {
          e.currentTarget.style.display = "none";
        }}
        className={cn("select-none", "pointer-events-none", className)}
      />
    );
  }
  return (
    <span
      data-identity-icon
      aria-hidden
      // The inlined svg fills the span; presentation-attribute sizes
      // on the svg lose to CSS.
      className={cn(
        "block",
        "select-none",
        "pointer-events-none",
        "[&>svg]:w-full",
        "[&>svg]:h-full",
        "[&>svg]:block",
        className,
      )}
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}
