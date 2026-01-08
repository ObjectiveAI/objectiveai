"use client";

import cn from "classnames";
import {
  ReactElement,
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { MarkdownContent } from "./Markdown";

// maximizes size until wrapping occurs
export function DynamicallySizedMarkdownContent({
  content,
  hiddenClassName,
}: {
  content: string;
  hiddenClassName?: string;
}): ReactElement {
  const [size, setSize] = useState<"md" | "lg" | "xl">("md");

  // store a reference to the hidden size detectors
  const hiddenLgRef = useRef<HTMLDivElement | null>(null);
  const hiddenXlRef = useRef<HTMLDivElement | null>(null);

  // update size based on the hidden detectors
  const updateSize = useCallback(() => {
    if (!hiddenLgRef.current || !hiddenXlRef.current) {
      return;
    } else if (
      hiddenXlRef.current.getBoundingClientRect().height <
      2 * 2.625 * 16
    ) {
      setSize("xl");
    } else if (
      hiddenLgRef.current.getBoundingClientRect().height <
      2 * 1.875 * 16
    ) {
      setSize("lg");
    } else {
      setSize("md");
    }
  }, []);

  // update size when content changes
  useLayoutEffect(updateSize, [updateSize, content]);

  // update size when hidden detectors resize
  useEffect(() => {
    if (!hiddenLgRef.current || !hiddenXlRef.current) return;
    const observer = new ResizeObserver(updateSize);
    observer.observe(hiddenLgRef.current);
    observer.observe(hiddenXlRef.current);
    return () => observer.disconnect();
  }, [updateSize]);

  return (
    <>
      <div
        ref={hiddenLgRef}
        className={cn(
          "fixed",
          "left-0",
          "top-0",
          "invisible",
          "pointer-events-none",
          "[overflow-wrap:anywhere]",
          "text-lg",
          "leading-[1.875rem]",
          "sm:text-xl",
          "sm:leading-8",
          hiddenClassName
        )}
      >
        {content}
      </div>
      <div
        ref={hiddenXlRef}
        className={cn(
          "fixed",
          "left-0",
          "top-0",
          "invisible",
          "pointer-events-none",
          "[overflow-wrap:anywhere]",
          "text-2xl",
          "leading-[2.25rem]",
          "sm:text-3xl",
          "sm:leading-[2.625rem]",
          hiddenClassName
        )}
      >
        {content}
      </div>
      <MarkdownContent content={content} size={size} />
    </>
  );
}
