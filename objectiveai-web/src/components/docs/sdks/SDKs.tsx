import { SharedHeader } from "@/components/SharedHeader";
import { SharedFooter } from "@/components/SharedFooter";
import { MarkdownContent } from "@/components/Markdown";
import { Provider } from "@/provider";
import { ReactElement } from "react";
import cn from "classnames";
import { Sidebar } from "@/components/docs/Sidebar";
import Link from "next/link";

const content = `Set up your development environment to use the ObjectiveAI API with an Open-Source SDK in your preferred language.`;

export function SDKs({
  session,
}: {
  session?: Provider.TokenSession;
}): ReactElement {
  return (
    <main className={cn("h-[100dvh]", "w-[100dvw]", "flex", "flex-col")}>
      <SharedHeader session={session} />
      <div className={cn("flex-grow", "flex", "overflow-hidden", "mt-1")}>
        <Sidebar />
        <div
          className={cn(
            "flex-grow",
            "flex",
            "flex-col",
            "overflow-auto",
            "basis-0",
            "px-1"
          )}
        >
          <div
            className={cn(
              "w-[calc(var(--spacing)*192)]",
              "max-w-full",
              "space-y-8",
              "mx-auto",
              "my-8"
            )}
          >
            <div className={cn("space-y-4")}>
              <h3
                className={cn(
                  "text-2xl",
                  "sm:text-3xl",
                  "font-medium",
                  "border-b",
                  "border-muted-secondary",
                  "pb-4"
                )}
              >
                SDKs
              </h3>
              <MarkdownContent content={content} />
            </div>
            <div className={cn("space-y-4")}>
              <h3
                className={cn(
                  "text-2xl",
                  "sm:text-3xl",
                  "font-medium",
                  "border-b",
                  "border-muted-secondary",
                  "pb-4"
                )}
              >
                Javascript / Typescript
              </h3>
              <Link
                href="https://www.npmjs.com/package/objectiveai"
                className={cn(
                  "block",
                  "w-fit",
                  "text-primary",
                  "text-base",
                  "sm:text-lg",
                  "hover:text-highlight-primary",
                  "transition-colors",
                  "duration-200"
                )}
                inert
              >
                npm
              </Link>
              <Link
                href="https://github.com/ObjectiveAI/objective_ai_js"
                className={cn(
                  "block",
                  "w-fit",
                  "text-primary",
                  "text-base",
                  "sm:text-lg",
                  "hover:text-highlight-primary",
                  "transition-colors",
                  "duration-200",
                  "border-b"
                )}
              >
                GitHub
              </Link>
            </div>
            <div className={cn("space-y-4")}>
              <h3
                className={cn(
                  "text-2xl",
                  "sm:text-3xl",
                  "font-medium",
                  "border-b",
                  "border-muted-secondary",
                  "pb-4"
                )}
              >
                Python
              </h3>
              <Link
                href=""
                className={cn(
                  "block",
                  "w-fit",
                  "text-primary",
                  "text-base",
                  "sm:text-lg",
                  "hover:text-highlight-primary",
                  "transition-colors",
                  "duration-200"
                )}
                inert
              >
                PyPI (Coming Soon)
              </Link>
              <Link
                href=""
                className={cn(
                  "block",
                  "w-fit",
                  "text-primary",
                  "text-base",
                  "sm:text-lg",
                  "hover:text-highlight-primary",
                  "transition-colors",
                  "duration-200"
                )}
                inert
              >
                GitHub (Coming Soon)
              </Link>
            </div>
            <div className={cn("space-y-4")}>
              <h3
                className={cn(
                  "text-2xl",
                  "sm:text-3xl",
                  "font-medium",
                  "border-b",
                  "border-muted-secondary",
                  "pb-4"
                )}
              >
                C# / .NET
              </h3>
              <Link
                href=""
                className={cn(
                  "block",
                  "w-fit",
                  "text-primary",
                  "text-base",
                  "sm:text-lg",
                  "hover:text-highlight-primary",
                  "transition-colors",
                  "duration-200"
                )}
                inert
              >
                NuGet (Coming Soon)
              </Link>
              <Link
                href=""
                className={cn(
                  "block",
                  "w-fit",
                  "text-primary",
                  "text-base",
                  "sm:text-lg",
                  "hover:text-highlight-primary",
                  "transition-colors",
                  "duration-200"
                )}
                inert
              >
                GitHub (Coming Soon)
              </Link>
            </div>
            <div className={cn("space-y-4")}>
              <h3
                className={cn(
                  "text-2xl",
                  "sm:text-3xl",
                  "font-medium",
                  "border-b",
                  "border-muted-secondary",
                  "pb-4"
                )}
              >
                Go
              </h3>
              <Link
                href=""
                className={cn(
                  "block",
                  "w-fit",
                  "text-primary",
                  "text-base",
                  "sm:text-lg",
                  "hover:text-highlight-primary",
                  "transition-colors",
                  "duration-200"
                )}
                inert
              >
                GitHub (Coming Soon)
              </Link>
            </div>
            <div className={cn("space-y-4")}>
              <h3
                className={cn(
                  "text-2xl",
                  "sm:text-3xl",
                  "font-medium",
                  "border-b",
                  "border-muted-secondary",
                  "pb-4"
                )}
              >
                Rust
              </h3>
              <Link
                href=""
                className={cn(
                  "block",
                  "w-fit",
                  "text-primary",
                  "text-base",
                  "sm:text-lg",
                  "hover:text-highlight-primary",
                  "transition-colors",
                  "duration-200"
                )}
                inert
              >
                crates.io (Coming Soon)
              </Link>
              <Link
                href=""
                className={cn(
                  "block",
                  "w-fit",
                  "text-primary",
                  "text-base",
                  "sm:text-lg",
                  "hover:text-highlight-primary",
                  "transition-colors",
                  "duration-200"
                )}
                inert
              >
                GitHub (Coming Soon)
              </Link>
            </div>
            <div className={cn("space-y-4")}>
              <h3
                className={cn(
                  "text-2xl",
                  "sm:text-3xl",
                  "font-medium",
                  "border-b",
                  "border-muted-secondary",
                  "pb-4"
                )}
              >
                Java
              </h3>
              <Link
                href=""
                className={cn(
                  "block",
                  "w-fit",
                  "text-primary",
                  "text-base",
                  "sm:text-lg",
                  "hover:text-highlight-primary",
                  "transition-colors",
                  "duration-200"
                )}
                inert
              >
                Maven Central (Coming Soon)
              </Link>
              <Link
                href=""
                className={cn(
                  "block",
                  "w-fit",
                  "text-primary",
                  "text-base",
                  "sm:text-lg",
                  "hover:text-highlight-primary",
                  "transition-colors",
                  "duration-200"
                )}
                inert
              >
                GitHub (Coming Soon)
              </Link>
            </div>
          </div>
        </div>
      </div>
      <SharedFooter />
    </main>
  );
}
