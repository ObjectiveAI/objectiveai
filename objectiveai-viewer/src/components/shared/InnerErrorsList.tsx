import * as Collapsible from "@radix-ui/react-collapsible";
import type { InnerError } from "../../lib/innerErrors";

export function InnerErrorsList({ errors }: { errors: InnerError[] }) {
  if (errors.length === 0) return null;

  return (
    <Collapsible.Root className="max-w-content mx-auto mb-4">
      <Collapsible.Trigger className="group flex items-center gap-2 px-4 py-1.5 w-full text-left text-[10px] font-mono text-amber-400 hover:text-amber-300 transition-colors select-none cursor-pointer">
        <svg
          className="w-2 h-2 transition-transform duration-150 group-data-[state=open]:rotate-90"
          viewBox="0 0 8 8"
          fill="currentColor"
        >
          <path d="M2 1l4 3-4 3z" />
        </svg>
        {errors.length} inner {errors.length === 1 ? "error" : "errors"}
      </Collapsible.Trigger>
      <Collapsible.Content>
        <div className="mx-4 rounded-md border border-amber-500/20 bg-amber-500/5 overflow-hidden">
          {errors.map((err, i) => (
            <div
              key={`${err.path}-${i}`}
              className="flex items-baseline gap-3 px-3 py-1.5 text-[10px] font-mono border-b border-amber-500/10 last:border-b-0"
            >
              <span className="text-info-dim shrink-0">{err.path}</span>
              <span className="text-amber-400 shrink-0">{err.code}</span>
              <span className="text-info-mid truncate">{err.message}</span>
            </div>
          ))}
        </div>
      </Collapsible.Content>
    </Collapsible.Root>
  );
}
