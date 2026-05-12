import type { ReactNode } from "react";
import * as ScrollArea from "@radix-ui/react-scroll-area";

export function Shell({ children, statusBar }: { children: ReactNode; statusBar?: ReactNode }) {
  return (
    <div className="flex flex-col h-screen">
      <header className="flex items-center gap-3 px-6 py-3 border-b border-node-border bg-ground-raised shrink-0">
        <div className="w-2 h-2 rounded-full bg-copper-hot" />
        <h1 className="font-mono text-sm font-semibold text-info-bright tracking-wide">ObjectiveAI Viewer</h1>
      </header>
      <ScrollArea.Root className="flex-1 overflow-hidden">
        <ScrollArea.Viewport className="h-full w-full">
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
