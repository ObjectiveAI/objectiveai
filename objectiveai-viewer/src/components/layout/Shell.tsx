import { useRef, useEffect, useCallback, type ReactNode } from "react";
import * as ScrollArea from "@radix-ui/react-scroll-area";
import { LogoMark, Wordmark } from "../shared/Logo";

export function Shell({ children, statusBar, entryCount }: { children: ReactNode; statusBar?: ReactNode; entryCount?: number }) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const userScrolledUp = useRef(false);

  const onScroll = useCallback(() => {
    const el = viewportRef.current;
    if (!el) return;
    userScrolledUp.current = el.scrollTop + el.clientHeight < el.scrollHeight - 100;
  }, []);

  useEffect(() => {
    if (userScrolledUp.current) return;
    const el = viewportRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [entryCount]);

  return (
    <div className="flex flex-col h-screen">
      <header className="flex items-center gap-2.5 px-6 py-3 border-b border-node-border bg-ground-raised shrink-0 select-none">
        <LogoMark className="h-5 w-auto text-info-bright" />
        <Wordmark className="h-3.5 w-auto text-info-bright" />
        <span className="text-info-dim text-[10px] uppercase tracking-widest font-mono ml-1">viewer</span>
      </header>
      <ScrollArea.Root className="flex-1 overflow-hidden">
        <ScrollArea.Viewport ref={viewportRef} className="h-full w-full" onScroll={onScroll}>
          <main className="py-6 px-4">
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
      {statusBar}
    </div>
  );
}
