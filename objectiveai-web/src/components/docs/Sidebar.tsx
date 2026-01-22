"use client";

import cn from "classnames";
import Link from "next/link";
import { ReactElement, ReactNode, useEffect, useState } from "react";
import { ArrowRightWideIcon, CodeSlashIcon, QuestionIcon } from "../Icon";

export function Sidebar({ className }: { className?: string }): ReactElement {
  const [open, setOpen] = useState(true);
  useEffect(() => {
    if (window.innerWidth < 640) setOpen(false);
  }, []);
  return (
    <div className={cn("flex", className)}>
      <aside
        className={cn("overflow-y-auto", "pr-1", "mr-3", !open && "hidden")}
      >
        <table className={cn("space-y-0.5", "my-8")}>
          <SidebarSection title="Overview" />
          <SidebarItem
            href="/docs"
            title="FAQ"
            description="Frequently Asked Questions"
          >
            {({ className }) => <QuestionIcon className={className} />}
          </SidebarItem>
          <SidebarItemSpacer />
          <SidebarItem
            href="/docs/sdks"
            title="SDKs"
            description="Use an Official Open-Source ObjectiveAI SDK"
          >
            {({ className }) => <CodeSlashIcon className={className} />}
          </SidebarItem>

          <SidebarSectionSpacer />

          <SidebarSection title="Ensemble LLMs" />
          <SidebarApiItem
            method="GET"
            path="/ensemble_llms"
            href="/docs/api/get/ensemble_llms"
            description="List all ObjectiveAI Ensemble LLMs"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="GET"
            path="/ensemble_llms/{id}"
            href="/docs/api/get/ensemble_llms/id"
            description="Retrieve an ObjectiveAI Ensemble LLM"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="GET"
            path="/ensemble_llms/{id}/usage"
            href="/docs/api/get/ensemble_llms/id/usage"
            description="Retrieve historical usage for an ObjectiveAI Ensemble LLM"
          />

          <SidebarSectionSpacer />

          <SidebarSection title="Ensembles" />
          <SidebarApiItem
            method="GET"
            path="/ensembles"
            href="/docs/api/get/ensembles"
            description="List all ObjectiveAI Ensembles"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="GET"
            path="/ensembles/{id}"
            href="/docs/api/get/ensembles/id"
            description="Retrieve an ObjectiveAI Ensemble"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="GET"
            path="/ensembles/{id}/usage"
            href="/docs/api/get/ensembles/id/usage"
            description="Retrieve historical usage for an ObjectiveAI Ensemble"
          />

          <SidebarSectionSpacer />

          <SidebarSection title="Functions" />
          <SidebarApiItem
            method="GET"
            path="/functions"
            href="/docs/api/get/functions"
            description="List all remote ObjectiveAI Functions"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="GET"
            path="/functions/{fowner}/{frepository}/{fcommit}"
            href="/docs/api/get/functions/fowner/frepository/fcommit"
            description="Retrieve a remote ObjectiveAI Function"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="GET"
            path="/functions/{fowner}/{frepository}/{fcommit}/usage"
            href="/docs/api/get/functions/fowner/frepository/fcommit/usage"
            description="Retrieve historical usage for a remote ObjectiveAI Function"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="POST"
            path="/functions"
            href="/docs/api/post/functions"
            description="Execute an inline ObjectiveAI Function with an inline Profile"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="POST"
            path="/functions/{fowner}/{frepository}/{fcommit}"
            href="/docs/api/post/functions/fowner/frepository/fcommit"
            description="Execute a remote ObjectiveAI Function with an inline Profile"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="POST"
            path="/functions/profiles/{powner}/{prepository}/{pcommit}"
            href="/docs/api/post/functions/profiles/powner/prepository/pcommit"
            description="Execute an inline ObjectiveAI Function with a remote Profile"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="POST"
            path="/functions/{fowner}/{frepository}/{fcommit}/profiles/{powner}/{prepository}/{pcommit}"
            href="/docs/api/post/functions/fowner/frepository/fcommit/profiles/powner/prepository/pcommit"
            description="Execute a remote ObjectiveAI Function with a remote Profile"
          />

          <SidebarSectionSpacer />

          <SidebarSection title="Function Profiles" />
          <SidebarApiItem
            method="POST"
            path="/functions/{fowner}/{frepository}/{fcommit}/profiles/compute"
            href="/docs/api/post/functions/fowner/frepository/fcommit/profiles/compute"
            description="Compute an ObjectiveAI Function Profile from your own Data (remote Function)"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="POST"
            path="/functions/profiles/compute"
            href="/docs/api/post/functions/profiles/compute"
            description="Compute an ObjectiveAI Function Profile from your own Data (inline Function)"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="GET"
            path="/functions/profiles"
            href="/docs/api/get/functions/profiles"
            description="List all remote ObjectiveAI Function Profiles"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="GET"
            path="/functions/profiles/{powner}/{prepository}/{pcommit}"
            href="/docs/api/get/functions/profiles/powner/prepository/pcommit"
            description="Retrieve a remote ObjectiveAI Function Profile"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="GET"
            path="/functions/profiles/{powner}/{prepository}/{pcommit}/usage"
            href="/docs/api/get/functions/profiles/powner/prepository/pcommit/usage"
            description="Retrieve historical usage for a remote ObjectiveAI Function Profile"
          />

          <SidebarSectionSpacer />

          <SidebarSection title="Vector" />
          <SidebarApiItem
            method="POST"
            path="/vector/completions"
            href="/docs/api/post/vector/completions"
            description="Create a new Vector Completion"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="GET"
            path="/vector/completions/{id}"
            href="/docs/api/get/vector/completions/id"
            description="Retrieve votes from a historical Vector Completion"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="GET"
            path="/vector/completions/cache"
            href="/docs/api/get/vector/completions/cache"
            description="Request a cached vote from the global ObjectiveAI vote cache"
          />

          <SidebarSectionSpacer />

          <SidebarSection title="Chat" />
          <SidebarApiItem
            method="POST"
            path="/chat/completions"
            href="/docs/api/post/chat/completions"
            description="Create a new Chat Completion"
          />

          <SidebarSectionSpacer />

          <SidebarSection title="Auth" />
          <SidebarApiItem
            method="GET"
            path="/auth/credits"
            href="/docs/api/get/auth/credits"
            description="Retrieve your available credits"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="GET"
            path="/auth/keys"
            href="/docs/api/get/auth/keys"
            description="List your API keys"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="POST"
            path="/auth/keys"
            href="/docs/api/post/auth/keys"
            description="Create a new API key"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="DELETE"
            path="/auth/keys"
            href="/docs/api/delete/auth/keys"
            description="Disable an API key"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="GET"
            path="/auth/keys/openrouter"
            href="/docs/api/get/auth/keys/openrouter"
            description="Retrieve your BYOK OpenRouter API key"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="POST"
            path="/auth/keys/openrouter"
            href="/docs/api/post/auth/keys/openrouter"
            description="Set your BYOK OpenRouter API key"
          />
          <SidebarItemSpacer />
          <SidebarApiItem
            method="DELETE"
            path="/auth/keys/openrouter"
            href="/docs/api/delete/auth/keys/openrouter"
            description="Remove your BYOK OpenRouter API key"
          />
        </table>
      </aside>
      <SidebarSlider onClick={() => setOpen((prev) => !prev)} open={open} />
    </div>
  );
}

function SidebarSection({ title }: { title: string }): ReactElement {
  return (
    <tbody>
      <tr>
        <FirstColumn />
        <td
          className={cn("text-base", "lg:text-lg", "font-bold", "text-primary")}
          colSpan={2}
        >
          <div
            className={cn(
              "mx-0.5",
              "lg:mx-1",
              "mb-1",
              "border-b",
              "border-muted-secondary",
              "px-2"
            )}
          >
            {title}
          </div>
        </td>
        <LastColumn />
      </tr>
    </tbody>
  );
}

function SidebarItem({
  children,
  title,
  href,
  description,
}: {
  children?: ({ className }: { className: string }) => ReactNode;
  title: string;
  href: string;
  description?: string;
}): ReactElement {
  return (
    <tbody className={cn("group")}>
      <tr>
        <FirstColumn />
        <td className={cn("w-0")}>
          <Link
            href={href}
            className={cn(
              "flex",
              "items-center",
              "justify-center",
              "w-full",
              "pr-2",
              "lg:pr-3",
              "text-xs",
              "lg:text-sm",
              "whitespace-nowrap"
            )}
          >
            {children?.({ className: cn("h-3", "lg:h-4") })}
          </Link>
        </td>
        <td>
          <Link
            href={href}
            className={cn(
              "flex",
              "items-center",
              "w-full",
              "text-sm",
              "lg:text-base",
              "font-bold",
              "text-primary",
              "group-hover:text-highlight-primary",
              "whitespace-nowrap"
            )}
          >
            {title}
          </Link>
        </td>
        <LastColumn />
      </tr>
      {description && (
        <tr>
          <FirstColumn />
          <td
            className={cn("w-0", "text-xs", "lg:text-sm", "text-secondary")}
            colSpan={2}
          >
            <Link
              href={href}
              className={cn(
                "flex",
                "w-full",
                "text-xs",
                "lg:text-sm",
                "text-secondary",
                "px-4"
              )}
            >
              {description}
            </Link>
          </td>
          <LastColumn />
        </tr>
      )}
    </tbody>
  );
}

type ApiMethod = "GET" | "POST" | "DELETE";

const methodColorMap = {
  GET: "bg-green-500/10 text-green-500",
  POST: "bg-blue-500/10 text-blue-500",
  DELETE: "bg-red-500/10 text-red-500",
};

function SidebarApiItem({
  method,
  path,
  href = path,
  description,
}: {
  method: ApiMethod;
  path: string;
  href?: string;
  description?: string;
}): ReactElement {
  return (
    <tbody className={cn("group")}>
      <tr>
        <FirstColumn />
        <td className={cn("w-0")}>
          <Link
            href={href}
            className={cn(
              "flex",
              "items-center",
              "justify-center",
              "w-full",
              "pr-2",
              "lg:pr-3",
              "font-mono",
              "text-xs",
              "lg:text-sm",
              "whitespace-nowrap"
            )}
          >
            <span
              className={cn(
                "px-1",
                "lg:px-1.5",
                "py-0.5",
                "rounded-lg",
                methodColorMap[method]
              )}
            >
              {method}
            </span>
          </Link>
        </td>
        <td>
          <Link
            href={href}
            className={cn(
              "flex",
              "items-center",
              "w-full",
              "text-sm",
              "lg:text-base",
              "font-bold",
              "text-primary",
              "group-hover:text-highlight-primary",
              "whitespace-nowrap"
            )}
          >
            {path}
          </Link>
        </td>
        <LastColumn />
      </tr>
      {description && (
        <tr>
          <FirstColumn />
          <td
            className={cn("w-0", "text-xs", "lg:text-sm", "text-secondary")}
            colSpan={2}
          >
            <Link
              href={href}
              className={cn(
                "flex",
                "w-full",
                "text-xs",
                "lg:text-sm",
                "text-secondary",
                "px-4"
              )}
            >
              {description}
            </Link>
          </td>
          <LastColumn />
        </tr>
      )}
    </tbody>
  );
}

function SidebarSectionSpacer(): ReactElement {
  return (
    <tbody>
      <tr>
        <FirstColumn />
        <td className={cn("h-4")} colSpan={2} />
        <LastColumn />
      </tr>
    </tbody>
  );
}

function SidebarItemSpacer(): ReactElement {
  return (
    <tbody>
      <tr>
        <FirstColumn />
        <td className={cn("h-2")} colSpan={2} />
        <LastColumn />
      </tr>
    </tbody>
  );
}

export function SidebarSlider({
  onClick,
  open = false,
  disabled = false,
  className,
}: {
  onClick: () => void;
  open?: boolean;
  disabled?: boolean;
  className?: string;
}): ReactElement {
  return (
    <div
      className={cn(
        open ? cn("w-3", "border-l") : "w-5",
        "relative",
        "h-full",
        "border-muted-secondary",
        className
      )}
    >
      <div
        className={cn(
          "absolute",
          "flex",
          "items-center",
          "bottom-0",
          "top-0",
          open ? "-translate-x-3" : "-translate-x-1"
        )}
      >
        <button
          className={cn(
            "block",
            "py-2",
            "text-muted-primary",
            "hover:text-primary",
            "bg-background-primary",
            disabled
              ? cn("cursor-not-allowed", "hover:bg-highlight-muted")
              : cn("cursor-pointer", "hover:bg-highlight-secondary"),
            "rounded-lg"
          )}
          onClick={onClick}
          disabled={disabled}
        >
          <ArrowRightWideIcon className={cn("h-6", open && "rotate-180")} />
        </button>
      </div>
    </div>
  );
}

function FirstColumn(): ReactElement {
  return <td className={cn("w-3", "lg:w-4")} />;
}

function LastColumn(): ReactElement {
  return <td className={cn("w-0")} />;
}
