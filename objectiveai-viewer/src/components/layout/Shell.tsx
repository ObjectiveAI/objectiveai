import { useState, useRef, useEffect, useCallback, type ReactNode } from "react";
import cn from "classnames";
import * as ScrollArea from "@radix-ui/react-scroll-area";
import { LogoMark, Wordmark } from "../shared/Logo";

export function Shell({ children, statusBar, banner, networkPanel, sidebar, detailPanel, entryCount }: { children: ReactNode; statusBar?: ReactNode; banner?: ReactNode; networkPanel?: ReactNode; sidebar?: ReactNode; detailPanel?: ReactNode; entryCount?: number }) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const userScrolledUp = useRef(false);
  const [showJumpToBottom, setShowJumpToBottom] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [detailVisible, setDetailVisible] = useState(false);
  const [detailClosing, setDetailClosing] = useState(false);
  const prevDetailPanel = useRef(detailPanel);

  useEffect(() => {
    if (detailPanel && !prevDetailPanel.current) {
      setDetailVisible(true);
      setDetailClosing(false);
    } else if (!detailPanel && prevDetailPanel.current) {
      setDetailClosing(true);
      const t = setTimeout(() => { setDetailVisible(false); setDetailClosing(false); }, 150);
      prevDetailPanel.current = detailPanel;
      return () => clearTimeout(t);
    }
    prevDetailPanel.current = detailPanel;
  }, [detailPanel]);

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

  useEffect(() => {
    if (!sidebar) return;
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "b") {
        e.preventDefault();
        setSidebarOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [sidebar]);

  return (
    <div className={cn("flex", "flex-col", "h-screen", "relative")}>
      <header role="banner" className={cn("flex", "items-center", "gap-2.5", "px-4", "py-2.5", "border-b", "border-node-border", "bg-ground-raised", "shrink-0", "select-none")}>
        {sidebar && (
          <button
            onClick={() => setSidebarOpen(!sidebarOpen)}
            aria-label="Toggle session sidebar"
            className={cn(
              "p-1.5",
              "rounded-sm",
              "transition-colors",
              sidebarOpen
                ? cn("text-copper-bright", "bg-copper-warm/15")
                : cn("text-info-dim", "hover:text-info-bright", "hover:bg-ground-surface"),
            )}
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
              <rect x="1" y="2" width="12" height="10" rx="1.5" />
              <path d="M5 2v10" />
            </svg>
          </button>
        )}
        <LogoMark className={cn("h-4.5", "w-auto", "text-info-bright")} />
        <Wordmark className={cn("w-[100px]", "h-auto", "text-info-bright")} />
        <span className={cn("text-info-dim", "text-[9px]", "uppercase", "tracking-widest", "font-mono", "ml-0.5")}>viewer</span>
        <div className={cn("ml-auto", "flex", "items-center", "gap-2")}>
          <kbd className={cn("px-1.5", "py-0.5", "rounded-sm", "bg-ground", "text-[10px]", "font-mono", "text-info-dim", "border", "border-node-border")}>{typeof navigator !== 'undefined' && navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}K</kbd>
        </div>
      </header>
      {banner}
      <div className={cn("flex", "flex-1", "min-h-0")}>
        {sidebar && (
          <aside className={cn(
              "shrink-0",
              "border-r",
              "border-node-border",
              "bg-ground-raised",
              "overflow-y-auto",
              "transition-[width]",
              "duration-200",
              "ease-out",
              sidebarOpen ? "w-56" : cn("w-0", "overflow-hidden"),
            )}>
            {sidebar}
          </aside>
        )}
        <div className={cn("flex-1", "flex", "flex-col", "min-w-0", "relative")}>
          <ScrollArea.Root className={cn("flex-1", "overflow-hidden")}>
            <ScrollArea.Viewport ref={viewportRef} className={cn("h-full", "w-full")} onScroll={onScroll}>
              <main role="main" className={cn("py-4", "px-4")}>
                {children}
              </main>
            </ScrollArea.Viewport>
            <ScrollArea.Scrollbar
              orientation="vertical"
              forceMount
              className={cn("flex", "select-none", "touch-none", "p-0.5", "bg-transparent", "transition-colors", "hover:bg-ground-surface/50", "w-2.5")}
            >
              <ScrollArea.Thumb className={cn("relative", "flex-1", "rounded-full", "bg-copper-dim/20", "hover:bg-copper-dim/50", "transition-colors")} />
            </ScrollArea.Scrollbar>
          </ScrollArea.Root>
          <div className={cn(
              "absolute",
              "bottom-16",
              "left-1/2",
              "-translate-x-1/2",
              "z-10",
              "transition-opacity",
              "duration-200",
              showJumpToBottom
                ? "opacity-100"
                : cn("opacity-0", "pointer-events-none"),
            )}>
            <button
              onClick={jumpToBottom}
              aria-label="Jump to latest entry"
              className={cn("flex", "items-center", "gap-1.5", "px-3", "py-1.5", "rounded-full", "bg-ground-raised/90", "backdrop-blur-sm", "border", "border-node-border", "text-[10px]", "font-mono", "text-info-mid", "hover:text-copper-bright", "hover:border-copper-dim", "shadow-lg", "transition-all")}
            >
              <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                <path d="M2 4L5 7L8 4" />
              </svg>
              Jump to latest
            </button>
          </div>
        </div>
        {(detailPanel || detailVisible) && (
          <aside className={cn(
              "shrink-0",
              "w-80",
              "overflow-y-auto",
              detailClosing ? "detail-panel-exit" : "detail-panel-enter",
            )}>
            {detailPanel ?? prevDetailPanel.current}
          </aside>
        )}
      </div>
      {networkPanel}
      {statusBar}
    </div>
  );
}
