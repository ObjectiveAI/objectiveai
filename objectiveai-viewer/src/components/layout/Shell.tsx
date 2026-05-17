import { useState, useRef, useEffect, useCallback, type ReactNode } from "react";
import * as ScrollArea from "@radix-ui/react-scroll-area";
import { LogoMark, Wordmark } from "../shared/Logo";

export function Shell({ children, statusBar, banner, networkPanel, entryCount }: { children: ReactNode; statusBar?: ReactNode; banner?: ReactNode; networkPanel?: ReactNode; entryCount?: number }) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const userScrolledUp = useRef(false);
  const [showJumpToBottom, setShowJumpToBottom] = useState(false);

  const onScroll = useCallback(() => {
    const el = viewportRef.current;
    if (!el) return;
    const isUp = el.scrollTop + el.clientHeight < el.scrollHeight - 100;
    userScrolledUp.current = isUp;
    setShowJumpToBottom(isUp);
  }, []);

  const jumpToBottom = useCallback(() => {
    const el = viewportRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
      userScrolledUp.current = false;
      setShowJumpToBottom(false);
    }
  }, []);

  useEffect(() => {
    if (userScrolledUp.current) return;
    const el = viewportRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [entryCount]);

  return (
    <div className="flex flex-col h-screen relative">
      <header role="banner" className="flex items-center gap-2.5 px-6 py-3 border-b border-node-border bg-ground-raised shrink-0 select-none">
        <LogoMark className="h-5 w-auto text-info-bright" />
        <Wordmark className="w-[110px] h-auto text-info-bright" />
        <span className="text-info-dim text-[10px] uppercase tracking-widest font-mono ml-1">viewer</span>
        <div className="ml-auto flex items-center gap-2">
          <kbd className="px-1.5 py-0.5 rounded-sm bg-ground text-[10px] font-mono text-info-dim border border-node-border">{typeof navigator !== 'undefined' && navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}K</kbd>
        </div>
      </header>
      {banner}
      <ScrollArea.Root className="flex-1 overflow-hidden">
        <ScrollArea.Viewport ref={viewportRef} className="h-full w-full" onScroll={onScroll}>
          <main role="main" className="py-6 px-4">
            {children}
          </main>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar
          orientation="vertical"
          className="flex select-none touch-none p-0.5 bg-transparent transition-colors hover:bg-ground-surface w-2.5"
        >
          <ScrollArea.Thumb className="relative flex-1 rounded-full bg-copper-dim/40 hover:bg-copper-dim/60" />
        </ScrollArea.Scrollbar>
      </ScrollArea.Root>
      {showJumpToBottom && (
        <div className="absolute bottom-20 left-1/2 -translate-x-1/2 z-10">
          <button
            onClick={jumpToBottom}
            aria-label="Jump to latest entry"
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-ground-raised/90 backdrop-blur-sm border border-node-border text-[10px] font-mono text-info-mid hover:text-copper-bright hover:border-copper-dim shadow-lg transition-all"
          >
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
              <path d="M2 4L5 7L8 4" />
            </svg>
            Jump to latest
          </button>
        </div>
      )}
      {networkPanel}
      {statusBar}
    </div>
  );
}
