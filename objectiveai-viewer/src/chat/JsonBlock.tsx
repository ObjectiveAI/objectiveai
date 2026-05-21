import cn from "classnames";
import JsonView from "@uiw/react-json-view";

interface JsonBlockProps {
  value: unknown;
}

/** Thin adapter over `@uiw/react-json-view`. Wrap so we can swap
 * libraries or theme without touching every call site. */
export function JsonBlock({ value }: JsonBlockProps) {
  return (
    <div
      className={cn(
        "rounded",
        "bg-neutral-100",
        "dark:bg-neutral-900",
        "p-2",
        "text-xs",
        "overflow-x-auto",
      )}
    >
      <JsonView value={value as object} collapsed={2} />
    </div>
  );
}
