import ReactMarkdown, { ExtraProps } from "react-markdown";
import cn from "classnames";
import remarkGfm from "remark-gfm";
import { ComponentType, isValidElement, JSX, ReactElement } from "react";

export function MarkdownContent({
  content,
  size = "md",
}: {
  content: string;
  size?: "md" | "lg" | "xl";
}): ReactElement {
  const strong:
    | ComponentType<JSX.IntrinsicElements["strong"] & ExtraProps>
    | keyof JSX.IntrinsicElements = ({
    node: _,
    className,
    children,
    ...props
  }) => (
    <strong className={cn("font-semibold", className)} {...props}>
      {children}
    </strong>
  );
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        strong,
        hr: ({ node: _, className, children, ...props }) => (
          <hr
            className={cn("my-10", "text-muted-secondary", className)}
            {...props}
          >
            {children}
          </hr>
        ),
        p: ({ node: _, className, children, ...props }) => {
          const [firstChildren, lastChild] = Array.isArray(children)
            ? [children.slice(0, -1), children[children.length - 1]]
            : [undefined, undefined];
          if (isValidElement(lastChild) && lastChild.type === strong) {
            return (
              <>
                <p
                  className={cn(
                    "mb-2",
                    "last:mb-0",
                    size === "md" &&
                      cn(
                        "text-sm",
                        "leading-[1.625rem]",
                        "sm:text-base",
                        "sm:leading-7"
                      ),
                    size === "lg" &&
                      cn(
                        "text-lg",
                        "leading-[1.875rem]",
                        "sm:text-xl",
                        "sm:leading-8"
                      ),
                    size === "xl" &&
                      cn(
                        "text-2xl",
                        "leading-[2.25rem]",
                        "sm:text-3xl",
                        "sm:leading-[2.625rem]"
                      ),
                    className
                  )}
                  {...props}
                >
                  {firstChildren}
                </p>
                <p
                  className={cn(
                    "mb-2",
                    size === "md" &&
                      cn(
                        "text-sm",
                        "leading-[1.625rem]",
                        "sm:text-base",
                        "sm:leading-7"
                      ),
                    size === "lg" &&
                      cn(
                        "text-lg",
                        "leading-[1.875rem]",
                        "sm:text-xl",
                        "sm:leading-8"
                      ),
                    size === "xl" &&
                      cn(
                        "text-2xl",
                        "leading-[2.25rem]",
                        "sm:text-3xl",
                        "sm:leading-[2.625rem]"
                      ),
                    className
                  )}
                  {...props}
                >
                  {lastChild}
                </p>
              </>
            );
          } else {
            return (
              <p
                className={cn(
                  "mb-2",
                  "last:mb-0",
                  size === "md" &&
                    cn(
                      "text-sm",
                      "leading-[1.625rem]",
                      "sm:text-base",
                      "sm:leading-7"
                    ),
                  size === "lg" &&
                    cn(
                      "text-lg",
                      "leading-[1.875rem]",
                      "sm:text-xl",
                      "sm:leading-8"
                    ),
                  size === "xl" &&
                    cn(
                      "text-2xl",
                      "leading-[2.25rem]",
                      "sm:text-3xl",
                      "sm:leading-[2.625rem]"
                    ),
                  className
                )}
                {...props}
              >
                {children}
              </p>
            );
          }
        },
        h3: ({ node: _, className, children, ...props }) => (
          <h3
            className={cn(
              "mt-4",
              "mb-2",
              "last:mb-0",
              size === "md" &&
                cn(
                  "text-lg",
                  "leading-[1.875rem]",
                  "sm:text-xl",
                  "sm:leading-8"
                ),
              size === "lg" &&
                cn(
                  "text-2xl",
                  "leading-9",
                  "sm:text-3xl",
                  "sm:leading-[2.625rem]"
                ),
              size === "xl" &&
                cn(
                  "text-4xl",
                  "leading-[3rem]",
                  "sm:text-5xl",
                  "sm:leading-[3.75rem]"
                ),
              "font-semibold",
              className
            )}
            {...props}
          >
            {children}
          </h3>
        ),
        h2: ({ node: _, className, children, ...props }) => (
          <h2
            className={cn(
              "mt-8",
              "mb-4",
              "last:mb-0",
              size === "md" &&
                cn("text-xl", "leading-7", "sm:text-2xl", "sm:leading-8"),
              size === "lg" &&
                cn(
                  "text-3xl",
                  "leading-[2.375rem]",
                  "sm:text-4xl",
                  "sm:leading-[2.75rem]"
                ),
              size === "xl" &&
                cn(
                  "text-5xl",
                  "leading-[3.5rem]",
                  "sm:text-6xl",
                  "sm:leading-[4.25rem]"
                ),
              "font-semibold",
              className
            )}
            {...props}
          >
            {children}
          </h2>
        ),
        h1: ({ node: _, className, children, ...props }) => (
          <h1
            className={cn(
              "mb-8",
              "last:mb-0",
              size === "md" &&
                cn(
                  "text-3xl",
                  "leading-[2.125rem]",
                  "sm:text-4xl",
                  "sm:leading-10"
                ),
              size === "lg" &&
                cn(
                  "text-5xl",
                  "leading-[3.25rem]",
                  "sm:text-6xl",
                  "sm:leading-[4rem]"
                ),
              size === "xl" &&
                cn(
                  "text-7xl",
                  "leading-[4.75rem]",
                  "sm:text-8xl",
                  "sm:leading-[6.25rem]"
                ),
              "font-bold",
              className
            )}
            {...props}
          >
            {children}
          </h1>
        ),
        ul: ({ node: _, className, children, ...props }) => (
          <ul
            className={cn("pl-6", "mb-4", "last:mb-0", "list-disc", className)}
            {...props}
          >
            {children}
          </ul>
        ),
        ol: ({ node: _, className, children, ...props }) => (
          <ol
            className={cn(
              "pl-6",
              "mb-4",
              "last:mb-0",
              "list-decimal",
              className
            )}
            {...props}
          >
            {children}
          </ol>
        ),
        li: ({ node: _, className, children, ...props }) => (
          <li className={cn("pl-1", "my-2", className)} {...props}>
            {children}
          </li>
        ),
        a: ({ node: _, className, children, ...props }) => {
          return (
            <a
              className={cn("text-blue-400", "hover:underline", className)}
              {...props}
            >
              {children}
            </a>
          );
        },
        table: ({ node: _, className, children, ...props }) => {
          return (
            <>
              <table
                className={cn(
                  size === "md" &&
                    cn(
                      "text-xs",
                      "leading-[1.375rem]",
                      "sm:text-sm",
                      "sm:leading-6"
                    ),
                  size === "lg" &&
                    cn(
                      "text-base",
                      "leading-[1.625rem]",
                      "sm:text-lg",
                      "sm:leading-7"
                    ),
                  size === "xl" &&
                    cn(
                      "text-xl",
                      "leading-[1.875rem]",
                      "sm:text-2xl",
                      "sm:leading-[2.125rem]"
                    ),
                  "w-full",
                  className
                )}
                {...props}
              >
                {children}
              </table>
              <div className={cn("h-3")} />
            </>
          );
        },
        th: ({ node: _, className, children, ...props }) => (
          <th
            className={cn(
              "text-left",
              "py-2",
              "pr-6",
              "last:pr-0",
              "pl-2",
              "first:pl-0",
              className
            )}
            {...props}
          >
            {children}
          </th>
        ),
        td: ({ node: _, className, children, ...props }) => (
          <td
            className={cn(
              "py-2.5",
              "pr-6",
              "last:pr-0",
              "pl-2",
              "first:pl-0",
              className
            )}
            {...props}
          >
            {children}
          </td>
        ),
        tr: ({ node: _, className, children, ...props }) => (
          <tr
            className={cn(
              "border-t",
              "first:border-t-0",
              "border-muted-secondary",
              className
            )}
            {...props}
          >
            {children}
          </tr>
        ),
        thead: ({ node: _, className, children, ...props }) => (
          <thead
            className={cn("border-b-2", "border-muted-secondary", className)}
            {...props}
          >
            {children}
          </thead>
        ),
        code: ({ node: _, className, children, ...props }) => {
          // Instead of relying on inline property, check the parent node type
          // If children is a string and doesn't contain newlines, it's likely inline
          const inline =
            typeof children === "string" && !children.includes("\n");
          // const language = className?.match(/language-(\w+)/)?.[1];
          // if (
          //   typeof children === "string" &&
          //   language &&
          //   SyntaxHighlighter.supportedLanguages.includes(language)
          // ) {
          //   return (
          //     <div
          //       className={cn(
          //         "p-2",
          //         "bg-background-code",
          //         "border",
          //         "border-muted-secondary",
          //         "rounded-lg",
          //         "w-fit",
          //         "min-w-min",
          //         "mb-2",
          //         className
          //       )}
          //     >
          //       {children}
          //     </div>
          //   );
          // } else if (inline) {
          if (inline) {
            return (
              <code
                className={cn(
                  "bg-background-code",
                  "px-1.5",
                  "py-0.5",
                  "rounded",
                  size === "md" &&
                    cn(
                      "text-xs",
                      "leading-[1.125rem]",
                      "sm:text-sm",
                      "sm:leading-5"
                    ),
                  size === "lg" &&
                    cn(
                      "text-base",
                      "leading-[1.375rem]",
                      "sm:text-lg",
                      "sm:leading-6"
                    ),
                  size === "xl" &&
                    cn(
                      "text-xl",
                      "leading-[1.625rem]",
                      "sm:text-2xl",
                      "sm:leading-[1.875rem]"
                    ),
                  className
                )}
                {...props}
              >
                {children}
              </code>
            );
          } else {
            return (
              <div
                className={cn(
                  "p-2",
                  "bg-background-code",
                  "rounded-lg",
                  "w-fit",
                  "min-w-min",
                  "mb-2",
                  size === "md" &&
                    cn(
                      "text-xs",
                      "leading-[1.125rem]",
                      "sm:text-sm",
                      "sm:leading-5"
                    ),
                  size === "lg" &&
                    cn(
                      "text-base",
                      "leading-[1.375rem]",
                      "sm:text-lg",
                      "sm:leading-6"
                    ),
                  size === "xl" &&
                    cn(
                      "text-xl",
                      "leading-[1.625rem]",
                      "sm:text-2xl",
                      "sm:leading-[1.875rem]"
                    ),
                  "whitespace-pre-wrap",
                  className
                )}
              >
                <code
                  style={
                    {
                      // whiteSpace: 'pre-wrap',
                    }
                  }
                  {...props}
                >
                  {children}
                </code>
              </div>
            );
          }
        },
      }}
    >
      {content}
    </ReactMarkdown>
  );
}
