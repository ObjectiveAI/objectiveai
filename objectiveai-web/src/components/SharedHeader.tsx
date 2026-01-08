import { Provider } from "@/provider";
import { ReactElement, ReactNode, Ref } from "react";
import cn from "classnames";
import Link from "next/link";
import { ObjectiveAIIcon } from "./Icon";
import { Legal, ThemeSelector, UserProfile } from "./SharedHeaderClient";

export function SharedHeader({
  session,
  ref,
  children,
  className,
}: {
  session?: Provider.TokenSession;
  ref?: Ref<HTMLDivElement | null>;
  children?: ReactNode;
  className?: string;
}): ReactElement {
  return (
    <div
      className={cn(
        "leading-5",
        "flex",
        "gap-x-3",
        "px-2",
        "pt-2",
        "whitespace-nowrap",
        className
      )}
      ref={ref}
    >
      <div
        className={cn(
          "flex-grow",
          "flex",
          "gap-y-1",
          "flex-wrap",
          "min-w-[calc(var(--spacing)*24)]",
          "items-center",
          "gap-x-3"
        )}
        ref={ref}
      >
        {children}
        <Link
          href="/"
          className={cn(
            "font-bold",
            "text-primary",
            "hover:text-highlight-primary"
          )}
        >
          <ObjectiveAIIcon className={cn("h-5", "sm:h-6")} />
        </Link>
        <span className={cn("space-x-3", "leading-none")}>
          <Link
            href="/docs"
            className={cn(
              "text-sm",
              "sm:text-base",
              "font-bold",
              "text-primary",
              "hover:text-highlight-primary"
            )}
          >
            Docs
          </Link>
          <Legal />
        </span>
      </div>
      <ThemeSelector />
      <UserProfile session={session} className={cn("min-w-fit")} />
    </div>
  );
}
